use crate::config::{Backend, Config};
use anyhow::{Context, Result};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Range;
use std::path::Path;
use std::time::Duration;
use tc_subxt::{MetadataVariant, SubxtClient, SubxtTxSubmitter};
use time_primitives::{
	AccountId, Address, ConnectorParams, Gateway, GmpEvent, GmpMessage, IConnector,
	IConnectorAdmin, MemberStatus, Network as Route, NetworkId, Runtime, ShardId, ShardStatus,
	TssPublicKey,
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
			let Some((conn_blockchain, conn_network)) = runtime.get_network(id).await? else {
				tracing::info!("network {id} not registered; skipping");
				continue;
			};
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

	fn connector(&self, network: NetworkId) -> Result<&dyn IConnectorAdmin> {
		Ok(&**self.connectors.get(&network).context("no connector configured for {network}")?)
	}

	async fn gateway(&self, network: NetworkId) -> Result<(&dyn IConnectorAdmin, Gateway)> {
		let connector = self.connector(network)?;
		let gateway = self
			.runtime
			.get_gateway(network)
			.await?
			.context("no gateway configured for {network}")?;
		Ok((connector, gateway))
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
				Ok(ShardStatus::Online) => {},
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

	pub fn parse_address(&self, network: Option<NetworkId>, address: &str) -> Result<Address> {
		if let Some(network) = network {
			self.connector(network)?.parse_address(address)
		} else {
			let address: AccountId =
				address.parse().map_err(|_| anyhow::anyhow!("invalid timechain account"))?;
			Ok(address.into())
		}
	}

	pub fn format_address(&self, network: Option<NetworkId>, address: Address) -> Result<String> {
		if let Some(network) = network {
			Ok(self.connector(network)?.format_address(address))
		} else {
			let address: AccountId = address.into();
			Ok(address.to_string())
		}
	}

	pub fn parse_balance(&self, network: Option<NetworkId>, balance: &str) -> Result<u128> {
		if let Some(network) = network {
			self.connector(network)?.parse_balance(balance)
		} else {
			Ok(balance.parse()?)
		}
	}

	pub fn format_balance(&self, network: Option<NetworkId>, balance: u128) -> Result<String> {
		if let Some(network) = network {
			Ok(self.connector(network)?.format_balance(balance))
		} else {
			Ok(balance.to_string())
		}
	}

	pub async fn faucet(&self, network: NetworkId) -> Result<()> {
		self.connector(network)?.faucet().await
	}

	pub async fn balance(&self, network: Option<NetworkId>, address: Address) -> Result<u128> {
		if let Some(network) = network {
			self.connector(network)?.balance(address).await
		} else {
			self.runtime.balance(&address.into()).await
		}
	}

	pub async fn transfer(
		&self,
		network: Option<NetworkId>,
		address: Address,
		balance: u128,
	) -> Result<()> {
		if let Some(network) = network {
			self.connector(network)?.transfer(address, balance).await?;
		} else {
			self.runtime.transfer(address.into(), balance).await?.wait_for_success().await?;
		}
		Ok(())
	}

	pub async fn fund(
		&self,
		network: Option<NetworkId>,
		address: Address,
		min_balance: u128,
	) -> Result<()> {
		let balance = self.balance(network, address).await?;
		let diff = min_balance.saturating_sub(balance);
		if diff > 0 {
			self.transfer(network, address, diff).await?;
		}
		Ok(())
	}
}

pub struct Network {
	pub network: NetworkId,
	pub chain_name: String,
	pub chain_network: String,
	pub gateway: Address,
	pub gateway_balance: u128,
	// TODO: pub block_height: u64,
	pub admin: Address,
	pub admin_balance: u128,
}

pub struct Chronicle {
	pub address: String,
	pub network: NetworkId,
	pub account: AccountId,
	pub peer_id: String,
	pub status: ChronicleStatus,
	pub balance: u128,
	pub target_address: Address,
	pub target_balance: u128,
}

struct ChronicleConfig {
	network: NetworkId,
	account: AccountId,
	peer_id: String,
	address: Address,
}

pub enum ChronicleStatus {
	Unregistered,
	Registered,
	Electable,
	Online,
}

impl std::fmt::Display for ChronicleStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let status = match self {
			Self::Unregistered => "unregistered",
			Self::Registered => "registered",
			Self::Electable => "electable",
			Self::Online => "online",
		};
		f.write_str(status)
	}
}

pub struct Shard {
	pub shard: ShardId,
	pub network: NetworkId,
	pub status: ShardStatus,
	pub key: Option<TssPublicKey>,
	pub registered: bool,
	pub size: u16,
	pub threshold: u16,
	// TODO: pub stake: u128,
}

pub struct Member {
	pub account: AccountId,
	pub status: MemberStatus,
	pub stake: u128,
}

impl Tc {
	pub async fn networks(&self) -> Result<Vec<Network>> {
		let network_ids = self.runtime.networks().await?;
		let mut networks = vec![];
		for network in network_ids {
			let (connector, gateway) = self.gateway(network).await?;
			let (chain_name, chain_network) =
				self.runtime.get_network(network).await?.context("invalid network")?;
			let gateway_balance = connector.balance(gateway).await?;
			let admin = connector.admin(gateway).await?;
			let admin_balance = connector.balance(admin).await?;
			networks.push(Network {
				network,
				chain_name,
				chain_network,
				gateway,
				gateway_balance,
				admin,
				admin_balance,
			});
		}
		Ok(networks)
	}

	pub async fn chronicles(&self) -> Result<Vec<Chronicle>> {
		let mut chronicles = vec![];
		for chronicle in &self.config.chronicles {
			let config = self.chronicle_config(chronicle).await?;
			let status = self.chronicle_status(&config.account).await?;
			let network = config.network;
			let balance = self.balance(None, config.account.clone().into()).await?;
			let target_balance = self.balance(Some(network), config.address).await?;
			chronicles.push(Chronicle {
				address: chronicle.clone(),
				network,
				account: config.account,
				peer_id: config.peer_id,
				status,
				balance,
				target_address: config.address,
				target_balance,
			})
		}
		Ok(chronicles)
	}

	async fn registered_shards(&self, network: NetworkId) -> Result<Vec<TssPublicKey>> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.shards(gateway).await
	}

	pub async fn shards(&self) -> Result<Vec<Shard>> {
		let shard_id_counter = self.runtime.shard_id_counter().await?;
		let mut shards = vec![];
		let mut registered_shards = HashMap::new();
		for shard in 0..shard_id_counter {
			let network = self.runtime.shard_network(shard).await?;
			if let Entry::Vacant(e) = registered_shards.entry(network) {
				e.insert(self.registered_shards(network).await?);
			}
			let status = self.runtime.get_shard_status(shard).await?;
			let key = self.runtime.get_shard_commitment(shard).await?.map(|c| c[0]);
			// TODO: get actual value not config value
			let size = self.runtime.shard_size().await?;
			let threshold = self.runtime.shard_threshold().await?;
			let mut registered = false;
			if let Some(key) = key {
				registered = registered_shards.get(&network).unwrap().contains(&key);
			}
			shards.push(Shard {
				shard,
				network,
				status,
				key,
				registered,
				size,
				threshold,
			});
		}
		Ok(shards)
	}

	pub async fn members(&self, shard: ShardId) -> Result<Vec<Member>> {
		let shard_members = self.runtime.get_shard_members(shard).await?;
		let mut members = Vec::with_capacity(shard_members.len());
		for (account, status) in shard_members {
			let stake = self.runtime.member_stake(&account).await?;
			members.push(Member { account, status, stake })
		}
		Ok(members)
	}

	pub async fn routes(&self, network: NetworkId) -> Result<Vec<Route>> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.networks(gateway).await
	}

	pub async fn events(&self, network: NetworkId, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.read_events(gateway, blocks).await
	}

	pub async fn messages(
		&self,
		network: NetworkId,
		tester: Address,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		let connector = self.connector(network)?;
		connector.recv_messages(tester, blocks).await
	}
}

impl Tc {
	async fn set_shard_config(&self) -> Result<()> {
		self.runtime
			.set_shard_config(self.config.config.shard_size, self.config.config.shard_threshold)
			.await?
			.wait_for_success()
			.await?;
		Ok(())
	}

	async fn register_network(&self, network: NetworkId) -> Result<Gateway> {
		let connector = self.connector(network)?;
		let config = self.config.network(network)?;
		let contracts = self.config.contracts(network)?;
		let (gateway, block) =
			connector.deploy_gateway(&contracts.proxy, &contracts.gateway).await?;
		self.runtime
			.register_network(
				network,
				config.blockchain.clone(),
				config.network.clone(),
				gateway,
				block,
			)
			.await?
			.wait_for_success()
			.await?;
		Ok(gateway)
	}

	async fn set_network_config(&self, network: NetworkId) -> Result<()> {
		let config = self.config.network(network)?;
		self.runtime
			.set_network_config(
				network,
				config.batch_size,
				config.batch_offset,
				config.batch_gas_limit,
				config.shard_task_limit,
			)
			.await?
			.wait_for_success()
			.await?;
		Ok(())
	}

	async fn set_electable(&self, accounts: Vec<AccountId>) -> Result<()> {
		self.runtime.set_electable(accounts).await?.wait_for_success().await?;
		Ok(())
	}

	async fn register_route(&self, network: NetworkId, route: Route) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.set_network(gateway, route).await?;
		Ok(())
	}

	async fn chronicle_config(&self, chronicle: &str) -> Result<ChronicleConfig> {
		let config: time_primitives::admin::Config =
			reqwest::get(format!("http://{chronicle}:8080/config")).await?.json().await?;
		Ok(ChronicleConfig {
			network: config.network,
			account: self.parse_address(None, &config.account)?.into(),
			address: self.parse_address(Some(config.network), &config.address)?,
			peer_id: config.peer_id,
		})
	}

	async fn chronicle_status(&self, account: &AccountId) -> Result<ChronicleStatus> {
		if self.runtime.member_network(account).await?.is_none() {
			return Ok(ChronicleStatus::Unregistered);
		}
		if !self.runtime.member_electable(account).await? {
			return Ok(ChronicleStatus::Registered);
		}
		if !self.runtime.member_online(account).await? {
			return Ok(ChronicleStatus::Electable);
		}
		Ok(ChronicleStatus::Online)
	}
}

impl Tc {
	pub async fn deploy(&self) -> Result<()> {
		self.set_shard_config().await?;
		for network in self.connectors.keys().copied() {
			let config = self.config.network(network)?;
			let gateway = self.register_network(network).await?;
			self.set_network_config(network).await?;
			self.fund(Some(network), gateway, config.gateway_funds).await?;
		}
		for src in self.connectors.keys().copied() {
			for dest in self.connectors.keys().copied() {
				let config = self.config.network(dest)?;
				let (_, gateway) = self.gateway(dest).await?;
				self.register_route(
					src,
					Route {
						network_id: dest,
						gateway,
						relative_gas_price: (
							config.route_gas_price.num,
							config.route_gas_price.den,
						),
						gas_limit: config.route_gas_limit,
						base_fee: config.route_base_fee,
					},
				)
				.await?;
			}
		}
		let mut accounts = Vec::with_capacity(self.config.chronicles.len());
		for chronicle in &self.config.chronicles {
			let chronicle = self.chronicle_config(chronicle).await?;
			self.fund(
				None,
				chronicle.account.clone().into(),
				self.config.config.chronicle_timechain_funds,
			)
			.await?;
			let config = self.config.network(chronicle.network)?;
			self.fund(Some(chronicle.network), chronicle.address, config.chronicle_target_funds)
				.await?;
			accounts.push(chronicle.account);
		}
		self.set_electable(accounts).await?;
		Ok(())
	}

	pub async fn register_shards(&self, network: NetworkId) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		let keys = self.find_online_shard_keys(network).await?;
		connector.set_shards(gateway, &keys).await
	}

	pub async fn set_gateway_admin(&self, network: NetworkId, admin: String) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.set_admin(gateway, connector.parse_address(&admin)?).await?;
		Ok(())
	}

	pub async fn redeploy_gateway(&self, network: NetworkId) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		let contracts = self.config.contracts(network)?;
		connector.redeploy_gateway(gateway, &contracts.gateway).await?;
		Ok(())
	}

	pub async fn deploy_tester(&self, network: NetworkId) -> Result<(Address, u64)> {
		let contracts = self.config.contracts(network)?;
		let (connector, gateway) = self.gateway(network).await?;
		connector.deploy_test(gateway, &contracts.tester).await
	}

	pub async fn send_message(
		&self,
		network: NetworkId,
		tester: Address,
		msg: GmpMessage,
	) -> Result<()> {
		let connector = self.connector(network)?;
		connector.send_message(tester, msg).await
	}
}
