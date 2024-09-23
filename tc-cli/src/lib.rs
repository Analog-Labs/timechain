use crate::config::{Backend, Config};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tabled::Tabled;
use tc_subxt::{MetadataVariant, SubxtClient, SubxtTxSubmitter};
use time_primitives::{
	Address, ConnectorParams, Gateway, IConnector, IConnectorAdmin, Network, NetworkId, Runtime,
	ShardStatus, TssPublicKey,
};

mod config;

async fn sleep_or_abort(duration: Duration) -> Result<()> {
	tokio::select! {
		_ = tokio::signal::ctrl_c() => {
			println!("aborting...");
			anyhow::bail!("abort");
		},
		_ = tokio::time::sleep(duration) => Ok(()),
	}
}

pub struct Tc {
	config: Config,
	runtime: SubxtClient,
	connectors: HashMap<NetworkId, Box<dyn IConnectorAdmin>>,
}

impl Tc {
	pub async fn new(
		config: &Path,
		keyfile: &Path,
		metadata: Option<&MetadataVariant>,
		url: &str,
	) -> Result<Self> {
		let config = Config::from_file(config)?;
		while SubxtClient::get_client(url).await.is_err() {
			tracing::info!("waiting for chain to start");
			sleep_or_abort(Duration::from_secs(10)).await?;
		}
		let tx_submitter = SubxtTxSubmitter::try_new(url).await.unwrap();
		let runtime = SubxtClient::with_key(
			url,
			metadata.cloned().unwrap_or_default(),
			&std::fs::read_to_string(keyfile)?,
			tx_submitter,
		)
		.await?;
		let mut connectors = HashMap::default();
		for (id, network) in &config.networks {
			let id = *id;
			let (conn_blockchain, conn_network) =
				runtime.get_network(id).await?.ok_or(anyhow::anyhow!("Unknown network id"))?;
			let params = ConnectorParams {
				network_id: id,
				blockchain: conn_blockchain,
				network: conn_network,
				url: network.url.clone(),
				mnemonic: std::fs::read_to_string(keyfile)?,
			};
			let connector = match network.backend {
				Backend::Rust => {
					let connector = gmp_rust::Connector::new(params).await?;
					Box::new(connector) as Box<dyn IConnectorAdmin>
				},
				Backend::Evm => {
					//let connector = gmp_evm::Connector::new(params).await?;
					//Box::new(connector)
					anyhow::bail!("evm backend not supported");
				},
			};
			connectors.insert(id, connector);
		}
		Ok(Self { config, runtime, connectors })
	}

	fn connector(&self, network: NetworkId) -> Result<&Box<dyn IConnectorAdmin>> {
		self.connectors.get(&network).context("no connector configured for {network}")
	}

	async fn find_online_shard_keys(&self, network: NetworkId) -> Result<Vec<TssPublicKey>> {
		let shard_id_counter = self.runtime.shard_id_counter().await?;
		let mut shards = vec![];
		for shard_id in 0..shard_id_counter {
			match self.runtime.shard_network(shard_id).await {
				Ok(shard_network) if shard_network == network => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			};
			match self.runtime.get_shard_status(shard_id).await {
				Ok(shard_status) if shard_status == ShardStatus::Online => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			}
			let shard_key = match self.runtime.get_shard_commitment(shard_id).await {
				Ok(Some(commitment)) => commitment[0],
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			};
			shards.push(shard_key);
		}
		Ok(shards)
	}

	pub fn parse_address(&self, network: NetworkId, address: &str) -> Result<Address> {
		self.connector(network)?.parse_address(address)
	}

	pub fn parse_balance(&self, network: NetworkId, balance: &str) -> Result<u128> {
		self.connector(network)?.parse_balance(balance)
	}
}

#[derive(Tabled)]
pub struct NetworkEntry {
	id: NetworkId,
	chain_name: String,
	chain_network: String,
	gateway: String,
	gateway_funds: String,
	gateway_admin: String,
	gateway_admin_funds: String,
	am_admin: String,
}

impl Tc {
	pub async fn networks(&self) -> Result<Vec<NetworkEntry>> {
		let network_id_counter = self.runtime.network_id_counter().await?;
		let mut networks = vec![];
		for id in 0..network_id_counter {
			let (chain_name, chain_network) =
				self.runtime.get_network(id).await?.context("invalid network id")?;
			let gateway_addr = self.runtime.get_gateway(id).await?;
			let mut gateway = None;
			let mut gateway_funds = None;
			let mut gateway_admin = None;
			let mut gateway_admin_funds = None;
			let mut am_admin = None;
			if let Some(gateway_addr) = gateway_addr {
				let gateway_addr = Address::from(gateway_addr);
				let connector = self.connector(id)?;
				let funds = connector.balance(gateway_addr).await?;
				let admin = connector.admin(gateway_addr).await?;
				let admin_funds = connector.balance(admin).await?;
				let my_account = connector.address();
				gateway = Some(connector.format_address(gateway_addr));
				gateway_funds = Some(connector.format_balance(funds));
				gateway_admin = Some(connector.format_address(admin));
				gateway_admin_funds = Some(connector.format_balance(admin_funds));
				am_admin = Some((admin == my_account).to_string());
			}
			networks.push(NetworkEntry {
				id,
				chain_name,
				chain_network,
				gateway: gateway.unwrap_or_default(),
				gateway_funds: gateway_funds.unwrap_or_default(),
				gateway_admin: gateway_admin.unwrap_or_default(),
				gateway_admin_funds: gateway_admin_funds.unwrap_or_default(),
				am_admin: am_admin.unwrap_or_default(),
			})
		}
		Ok(networks)
	}

	pub async fn add_network(&self, chain_name: String, chain_network: String) -> Result<()> {
		self.runtime
			.register_network(chain_name, chain_network)
			.await?
			.wait_for_success()
			.await?;
		Ok(())
	}
}

#[derive(Tabled)]
pub struct Routing {
	network: NetworkId,
	gateway: String,
	relative_gas_price: String,
	gas_limit: u64,
	base_fee: u128,
}

impl Tc {
	async fn gateway(&self, network: NetworkId) -> Result<(&Box<dyn IConnectorAdmin>, Gateway)> {
		let connector = self.connector(network)?;
		let gateway = self
			.runtime
			.get_gateway(network)
			.await?
			.context("no gateway configured for {network}")?;
		Ok((connector, gateway))
	}

	pub async fn gateway_routing(&self, network: NetworkId) -> Result<Vec<Routing>> {
		let (connector, gateway) = self.gateway(network).await?;
		let networks = connector.networks(gateway.into()).await?;
		let routing = networks
			.into_iter()
			.map(|network| {
				let (num, den) = network.relative_gas_price;
				Routing {
					network: network.network_id,
					gateway: connector.format_address(Address::from(network.gateway)),
					relative_gas_price: format!("{}", num as f64 / den as f64),
					gas_limit: network.gas_limit,
					base_fee: network.base_fee,
				}
			})
			.collect();
		Ok(routing)
	}

	pub async fn deploy_gateway(&self, network: NetworkId) -> Result<()> {
		let connector = self.connector(network)?;
		let backend = self.config.backend(network)?;
		if let Some(gateway) = self.runtime.get_gateway(network).await? {
			connector.redeploy_gateway(gateway.into(), &backend.gateway).await?;
		} else {
			let (gateway, block) =
				connector.deploy_gateway(&backend.proxy, &backend.gateway).await?;
			self.runtime
				.register_gateway(network, gateway.into(), block)
				.await?
				.wait_for_success()
				.await?;
		}
		Ok(())
	}

	pub async fn set_gateway_admin(&self, network: NetworkId, admin: String) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.set_admin(gateway.into(), connector.parse_address(&admin)?).await?;
		Ok(())
	}

	pub async fn register_shards(&self, network: NetworkId) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		let keys = self.find_online_shard_keys(network).await?;
		connector.set_shards(gateway, &keys).await
	}

	pub async fn set_gateway_routing(
		&self,
		network: NetworkId,
		network_info: Network,
	) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.set_network(gateway, network_info).await?;
		Ok(())
	}
}

#[derive(Tabled)]
pub struct Wallet {
	address: String,
	balance: String,
}

impl Tc {
	pub async fn wallet(&self, network: NetworkId) -> Result<Wallet> {
		let connector = self.connector(network)?;
		let address = connector.address();
		let balance = connector.balance(address).await?;
		Ok(Wallet {
			address: connector.format_address(address),
			balance: connector.format_balance(balance),
		})
	}

	pub async fn faucet(&self, network: NetworkId) -> Result<()> {
		self.connector(network)?.faucet().await
	}

	pub async fn transfer(&self, network: NetworkId, address: &str, amount: &str) -> Result<()> {
		let address = self.parse_address(network, address)?;
		let balance = self.parse_balance(network, amount)?;
		self.connector(network)?.transfer(address, balance).await
	}
}
