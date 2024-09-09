use anyhow::{Context, Result};
use gmp_evm::{Address, AdminConnector};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tabled::Tabled;
use tc_subxt::{MetadataVariant, SubxtClient, SubxtTxSubmitter};
use time_primitives::{IConnectorAdmin, Network, NetworkId, Runtime, ShardId, ShardStatus};

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
	runtime: SubxtClient,
	connectors: HashMap<NetworkId, AdminConnector>,
}

impl Tc {
	pub async fn new(
		keyfile: &Path,
		metadata: Option<&MetadataVariant>,
		url: &str,
	) -> Result<Self> {
		while SubxtClient::get_client(url).await.is_err() {
			tracing::info!("waiting for chain to start");
			sleep_or_abort(Duration::from_secs(10)).await?;
		}
		let tx_submitter = SubxtTxSubmitter::try_new(url).await.unwrap();
		let runtime = SubxtClient::with_keyfile(
			url,
			metadata.cloned().unwrap_or_default(),
			keyfile,
			tx_submitter,
		)
		.await?;
		Ok(Self {
			runtime,
			connectors: Default::default(),
		})
	}

	pub async fn add_connector(
		&mut self,
		id: NetworkId,
		url: &str,
		keyfile: &Path,
		gateway: &Path,
		proxy: &Path,
		tester: &Path,
	) -> Result<()> {
		let (conn_blockchain, conn_network) = self
			.runtime
			.get_network(id)
			.await?
			.ok_or(anyhow::anyhow!("Unknown network id"))?;
		let connector = AdminConnector::new(
			id,
			&conn_blockchain,
			&conn_network,
			&url,
			keyfile,
			gateway.into(),
			proxy.into(),
			tester.into(),
		)
		.await?;
		self.connectors.insert(id, connector);
		Ok(())
	}

	fn connector(&self, network: NetworkId) -> Result<&AdminConnector> {
		self.connectors.get(&network).context("no connector configured for {network}")
	}

	async fn find_online_shard(&self, network: NetworkId) -> Result<ShardId> {
		let shard_id_counter = self.runtime.shard_id_counter().await?;
		for shard_id in 0..shard_id_counter {
			match self.runtime.shard_network(shard_id).await {
				Ok(shard_network) if shard_network == network => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			};
			match self.runtime.shard_state(shard_id).await {
				Ok(shard_status) if shard_status == ShardStatus::Online => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			}
			return Ok(shard_id);
		}
		anyhow::bail!("no online shard for network {network}");
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
				gateway = Some(gateway_addr.to_string());
				gateway_funds = Some(funds.to_string());
				gateway_admin = Some(admin.to_string());
				gateway_admin_funds = Some(admin_funds.to_string());
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
	async fn gateway(&self, network: NetworkId) -> Result<(&AdminConnector, [u8; 20])> {
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
					gateway: Address::from(network.gateway).to_string(),
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
		if let Some(gateway) = self.runtime.get_gateway(network).await? {
			connector.redeploy_gateway(gateway.into()).await?;
		} else {
			let shard_id = self.find_online_shard(network).await?;
			let key = self.runtime.shard_public_key(shard_id).await?;
			let (gateway, block) = connector.deploy_gateway().await?;
			connector.set_shards(gateway, &[key]).await?;
			self.runtime
				.register_gateway(shard_id, gateway.into(), block)
				.await?
				.wait_for_success()
				.await?;
		}
		Ok(())
	}

	pub async fn set_gateway_admin(&self, network: NetworkId, admin: String) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.set_admin(gateway.into(), admin.parse()?).await?;
		Ok(())
	}

	pub async fn set_gateway_routing(
		&self,
		network: NetworkId,
		network_info: Network,
	) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.set_network(gateway.into(), network_info).await?;
		Ok(())
	}
}
