use crate::config::Config;
use crate::env::Mnemonics;
use crate::gas_price::{convert_bigint_to_u128, get_network_price};
use crate::table::IntoRow;
use anyhow::{Context, Result};
use futures::stream::{BoxStream, FuturesUnordered, StreamExt};
use polkadot_sdk::sp_runtime::BoundedVec;
use scale_codec::{Decode, Encode};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tc_subxt::SubxtClient;
use time_primitives::{
	balance::BalanceFormatter, traits::IdentifyAccount, AccountId, Address, BatchId, BlockHash,
	BlockNumber, ChainName, ChainNetwork, ConnectorParams, Gateway, GatewayMessage, GmpEvent,
	GmpEvents, GmpMessage, Hash, IConnectorAdmin, MemberStatus, MessageId, NetworkConfig,
	NetworkId, PeerId, PublicKey, Route, ShardId, ShardStatus, TaskId, TssPublicKey,
};
use tokio::time::sleep;

mod config;
mod env;
mod gas_price;
mod loki;
mod slack;
mod table;

pub use crate::loki::{Log, Query};
pub use crate::slack::{Sender, TableRef, TextRef};

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
	connectors: HashMap<NetworkId, Arc<dyn IConnectorAdmin>>,
	msg: Sender,
}

impl Tc {
	pub async fn new(env: PathBuf, config: &str, msg: Sender) -> Result<Self> {
		dotenv::from_path(env.join(".env")).ok();
		let config = Config::from_env(env, config)?;
		let env = Mnemonics::from_env()?;
		let timechain_url = config.global().timechain_url.clone();
		let runtime = tokio::task::spawn(async move {
			while let Err(err) = SubxtClient::get_client(&timechain_url).await {
				tracing::info!("waiting for chain to start: {err:?}");
				sleep_or_abort(Duration::from_secs(10)).await?;
			}
			let runtime =
				SubxtClient::with_key(&timechain_url, &env.timechain_mnemonic, "cached_tx.redb")
					.await
					.context("failed to connect to timechain")?;
			Ok::<_, anyhow::Error>(runtime)
		});
		let mut connectors = HashMap::new();
		{
			let mut connector_futures = FuturesUnordered::new();
			for (id, network) in config.networks() {
				let id = *id;
				let params = ConnectorParams {
					network_id: id,
					blockchain: network.blockchain.clone(),
					network: network.network.clone(),
					url: network.url.clone(),
					mnemonic: env.target_mnemonic.clone(),
					cctp_sender: None,
					cctp_attestation: None,
				};
				let connector = async move {
					let connector = network
						.backend
						.connect_admin(&params)
						.await
						.with_context(|| format!("failed to connect to backend {id}"))?;
					Ok::<_, anyhow::Error>((id, connector))
				};
				connector_futures.push(connector);
			}
			while let Some(res) = connector_futures.next().await {
				let (network, connector) = res?;
				connectors.insert(network, connector);
			}
		}
		let runtime = runtime.await??;
		Ok(Self {
			config,
			runtime,
			connectors,
			msg,
		})
	}

	fn connector(&self, network: NetworkId) -> Result<&dyn IConnectorAdmin> {
		Ok(&**self
			.connectors
			.get(&network)
			.with_context(|| format!("no connector configured for {network}"))?)
	}

	async fn gateway(&self, network: NetworkId) -> Result<(&dyn IConnectorAdmin, Gateway)> {
		let connector = self.connector(network)?;
		let gateway = self
			.runtime
			.network_gateway(network)
			.await?
			.with_context(|| format!("no gateway configured for {network}"))?;
		Ok((connector, gateway))
	}

	pub fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.runtime.finality_notification_stream()
	}

	pub async fn runtime_upgrade(&self, path: &Path) -> Result<()> {
		self.println(None, "runtime-upgrade").await?;
		let bytecode = std::fs::read(path)?;
		self.runtime.set_code(bytecode).await
	}

	pub async fn find_online_shard_keys(&self, network: NetworkId) -> Result<Vec<TssPublicKey>> {
		let shard_id_counter = self.runtime.shard_id_counter().await?;
		let mut shards = vec![];
		for shard_id in 0..shard_id_counter {
			match self.runtime.shard_network(shard_id).await {
				Ok(Some(shard_network)) if shard_network == network => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			};
			match self.runtime.shard_status(shard_id).await {
				Ok(ShardStatus::Online) => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			}
			let shard_key = match self.runtime.shard_public_key(shard_id).await {
				Ok(Some(key)) => key,
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
			let address: AccountId = address
				.parse()
				.map_err(|err| anyhow::anyhow!("invalid timechain account: {err}"))?;
			Ok(address.into())
		}
	}

	pub fn format_address(&self, network: Option<NetworkId>, address: Address) -> Result<String> {
		if let Some(network) = network {
			Ok(self.connector(network)?.format_address(address))
		} else {
			let address: AccountId = address.into();
			Ok(time_primitives::format_address(&address))
		}
	}

	pub fn currency(&self, network: Option<NetworkId>) -> Result<(u32, &str)> {
		if let Some(network) = network {
			Ok(self.connector(network)?.currency())
		} else {
			Ok((12, "ANLG"))
		}
	}

	pub fn parse_balance(&self, network: Option<NetworkId>, balance: &str) -> Result<u128> {
		if let Some(network) = network {
			self.connector(network)?.parse_balance(balance)
		} else {
			BalanceFormatter::new(12, "ANLG").parse(balance)
		}
	}

	pub fn format_balance(&self, network: Option<NetworkId>, balance: u128) -> Result<String> {
		if let Some(network) = network {
			Ok(self.connector(network)?.format_balance(balance))
		} else {
			Ok(BalanceFormatter::new(12, "ANLG").format(balance))
		}
	}

	pub fn address(&self, network: Option<NetworkId>) -> Result<Address> {
		Ok(if let Some(network) = network {
			self.connector(network)?.address()
		} else {
			self.runtime.account_id().clone().into()
		})
	}

	pub async fn faucet(&self, network: NetworkId) -> Result<()> {
		let config = self.config.network(network)?;
		let Some(admin_funds) = config.admin_funds.as_ref() else {
			return Ok(());
		};
		let admin_funds = self.parse_balance(Some(network), admin_funds)?;
		let current_admin_funds = self.balance(Some(network), self.address(Some(network))?).await?;
		let faucet = admin_funds.saturating_sub(current_admin_funds);
		if faucet == 0 {
			return Ok(());
		}
		self.println(
			None,
			format!("faucet {network} {}", self.format_balance(Some(network), faucet)?),
		)
		.await?;
		self.connector(network)?.faucet(faucet).await.with_context(|| {
			format!(
				"faucet failed or is unsupported, please transfer {} to {}",
				self.format_balance(Some(network), faucet).unwrap(),
				self.format_address(Some(network), self.address(Some(network)).unwrap())
					.unwrap(),
			)
		})
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
		self.println(
			None,
			format!(
				"transfering {} to {}",
				self.format_balance(network, balance)?,
				self.format_address(network, address)?,
			),
		)
		.await?;
		if let Some(network) = network {
			self.connector(network)?.transfer(address, balance).await?;
		} else {
			self.runtime.transfer(address.into(), balance).await?;
		}
		Ok(())
	}

	pub async fn fund(
		&self,
		network: Option<NetworkId>,
		address: Address,
		min_balance: u128,
		label: &str,
	) -> Result<()> {
		let balance = self.balance(network, address).await?;
		let diff = min_balance.saturating_sub(balance);
		if diff > 0 {
			self.println(None, format!("funding {label}")).await?;
			self.transfer(network, address, diff).await?;
		}
		Ok(())
	}
}

#[derive(Clone, Debug)]
pub struct Network {
	pub network: NetworkId,
	pub chain_name: String,
	pub chain_network: String,
	pub info: Option<NetworkInfo>,
}

#[derive(Clone, Debug)]
pub struct NetworkInfo {
	pub gateway: Address,
	pub gateway_balance: u128,
	pub admin: Address,
	pub admin_balance: u128,
	pub sync_status: SyncStatus,
	pub unassigned_tasks: u32,
}

#[derive(Clone, Debug)]
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
	public_key: PublicKey,
	peer_id: PeerId,
	peer_id_str: String,
	address: Address,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ChronicleStatus {
	Unregistered,
	Registered,
	Online,
}

impl std::fmt::Display for ChronicleStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let status = match self {
			Self::Unregistered => "unregistered",
			Self::Registered => "registered",
			Self::Online => "online",
		};
		f.write_str(status)
	}
}

#[derive(Clone, Debug)]
pub struct Shard {
	pub shard: ShardId,
	pub network: NetworkId,
	pub status: ShardStatus,
	pub key: Option<TssPublicKey>,
	pub registered: bool,
	pub size: u16,
	pub threshold: u16,
	// TODO: pub stake: u128,
	pub assigned: usize,
}

#[derive(Clone, Debug)]
pub struct Member {
	pub account: AccountId,
	pub status: MemberStatus,
	pub staker: Option<AccountId>,
	pub stake: u128,
}

#[derive(Clone, Debug)]
pub struct Task {
	pub task: TaskId,
	pub network: NetworkId,
	pub descriptor: time_primitives::Task,
	pub output: Option<Result<(), String>>,
	pub shard: Option<ShardId>,
	pub submitter: Option<PublicKey>,
}

#[derive(Clone, Debug)]
pub struct Batch {
	pub batch: BatchId,
	pub msg: GatewayMessage,
	pub task: TaskId,
	pub tx: Option<Hash>,
}

#[derive(Clone, Debug)]
pub struct Message {
	pub message: MessageId,
	pub recv: Option<TaskId>,
	pub batch: Option<BatchId>,
	pub exec: Option<TaskId>,
}

#[derive(Clone, Debug)]
pub struct SyncStatus {
	pub network: NetworkId,
	pub task: TaskId,
	pub block: u64,
	pub sync: u64,
	pub next_sync: u64,
}

#[derive(Clone, Debug)]
pub struct MessageTrace {
	pub message: MessageId,
	pub src: SyncStatus,
	pub dest: Option<SyncStatus>,
	pub recv: Option<Task>,
	pub submit: Option<Task>,
	pub exec: Option<Task>,
}

fn same<T: PartialEq>(a: &[T], b: &[T]) -> bool {
	if a.len() != b.len() {
		return false;
	}
	for a in a {
		if !b.contains(a) {
			return false;
		}
	}
	true
}

impl Tc {
	pub async fn read_events_blocks(&self, task: TaskId) -> Result<Range<u64>> {
		let task = self.runtime.task(task).await?.context("no read event task")?;
		let time_primitives::Task::ReadGatewayEvents { blocks } = task else {
			anyhow::bail!("invalid read event task descriptor");
		};
		Ok(blocks)
	}

	pub async fn sync_status(&self, network: NetworkId) -> Result<SyncStatus> {
		let sync_task =
			self.runtime.read_events_task(network).await?.context("no read events task")?;
		let blocks = self.read_events_blocks(sync_task).await?;
		let block = self
			.connector(network)?
			.finalized_block()
			.await
			.context("failed to read target block")?;
		Ok(SyncStatus {
			network,
			task: sync_task,
			block,
			sync: blocks.start,
			next_sync: blocks.end,
		})
	}

	pub async fn networks(&self) -> Result<Vec<Network>> {
		let network_ids = self.runtime.networks().await?;
		let mut networks = vec![];
		for network in network_ids {
			let (chain_name, chain_network) =
				self.runtime.network_name(network).await?.context("invalid network")?;
			let chain_name =
				String::decode(&mut chain_name.0.to_vec().as_slice()).unwrap_or_default();
			let chain_network =
				String::decode(&mut chain_network.0.to_vec().as_slice()).unwrap_or_default();
			let info = match self.gateway(network).await {
				Ok((connector, gateway)) => {
					let gateway_balance = connector.balance(gateway).await?;
					let admin = connector.admin(gateway).await?;
					let admin_balance = connector.balance(admin).await?;
					let sync_status = self.sync_status(network).await?;
					let unassigned_tasks = self.runtime.unassigned_tasks(network).await?;
					Some(NetworkInfo {
						gateway,
						gateway_balance,
						admin,
						admin_balance,
						sync_status,
						unassigned_tasks: unassigned_tasks.len() as _,
					})
				},
				Err(_) => None,
			};
			networks.push(Network {
				network,
				chain_name,
				chain_network,
				info,
			});
		}
		Ok(networks)
	}

	pub async fn chronicles(&self) -> Result<Vec<Chronicle>> {
		let mut chronicles = vec![];
		for chronicle in self.config.chronicles() {
			let config = self.chronicle_config(chronicle).await?;
			let status = self.chronicle_status(&config.account).await?;
			let network = config.network;
			let balance = self.balance(None, config.account.clone().into()).await?;
			let target_balance = self.balance(Some(network), config.address).await?;
			chronicles.push(Chronicle {
				address: chronicle.clone(),
				network,
				account: config.account,
				peer_id: config.peer_id_str,
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
			let Some(network) = self.runtime.shard_network(shard).await? else {
				continue;
			};
			if let Entry::Vacant(e) = registered_shards.entry(network) {
				e.insert(self.registered_shards(network).await?);
			}
			let status = self.runtime.shard_status(shard).await?;
			let key = self.runtime.shard_commitment(shard).await?.map(|c| c.0[0]);
			let size = self.runtime.shard_members(shard).await?.len() as u16;
			let threshold = self.runtime.shard_threshold(shard).await?;
			let mut registered = false;
			if let Some(key) = key {
				registered = registered_shards.get(&network).unwrap().contains(&key);
			}
			let assigned = self.runtime.assigned_tasks(shard).await?.len();
			shards.push(Shard {
				shard,
				network,
				status,
				key,
				registered,
				size,
				threshold,
				assigned,
			});
		}
		Ok(shards)
	}

	pub async fn unassigned_tasks(&self, network: NetworkId) -> Result<Vec<Task>> {
		let task_ids = self.runtime.unassigned_tasks(network).await?;
		let mut tasks = Vec::with_capacity(task_ids.len());
		for id in task_ids {
			tasks.push(self.task(id).await?);
		}
		Ok(tasks)
	}

	pub async fn assigned_tasks(&self, shard: ShardId) -> Result<Vec<Task>> {
		let task_ids = self.runtime.assigned_tasks(shard).await?;
		let mut tasks = Vec::with_capacity(task_ids.len());
		for id in task_ids {
			tasks.push(self.task(id).await?);
		}
		Ok(tasks)
	}

	pub async fn get_failed_batches(&self) -> Result<Vec<Batch>> {
		let batch_ids = self.runtime.get_failed_tasks().await?;
		let mut batches = Vec::with_capacity(batch_ids.len());
		for id in batch_ids {
			batches.push(self.batch(id).await?);
		}
		Ok(batches)
	}

	pub async fn members(&self, shard: ShardId) -> Result<Vec<Member>> {
		let shard_members = self.runtime.shard_members(shard).await?;
		let mut members = Vec::with_capacity(shard_members.len());
		for (account, status) in shard_members {
			let staker = self.runtime.member_staker(&account).await?;
			let stake = self.runtime.member_stake(&account).await?;
			members.push(Member { account, status, staker, stake })
		}
		Ok(members)
	}

	pub async fn routes(&self, network: NetworkId) -> Result<Vec<Route>> {
		let (connector, gateway) = self.gateway(network).await?;
		connector.routes(gateway).await
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

	pub async fn task(&self, task: TaskId) -> Result<Task> {
		Ok(Task {
			task,
			network: self.runtime.task_network(task).await?.context("invalid task id")?,
			descriptor: self.runtime.task(task).await?.context("invalid task id")?,
			output: self.runtime.task_output(task).await?.map(|o| {
				o.map_err(|e| String::decode(&mut e.0.to_vec().as_slice()).unwrap_or_default())
			}),
			shard: self.runtime.assigned_shard(task).await?,
			submitter: self.runtime.task_submitter(task).await?,
		})
	}

	pub async fn transaction_base_fee(&self, network: NetworkId) -> Result<u128> {
		let connector = self
			.connectors
			.get(&network)
			.with_context(|| format!("Connector for network id: {:?} not found", network))?;
		let base_fee = connector.transaction_base_fee().await?;
		Ok(base_fee)
	}

	pub async fn block_gas_limit(&self, network: NetworkId) -> Result<u64> {
		let connector = self
			.connectors
			.get(&network)
			.with_context(|| format!("Connector for network id: {:?} not found", network))?;
		let gas_limit = connector.block_gas_limit().await?;
		Ok(gas_limit)
	}

	pub async fn batch(&self, batch: BatchId) -> Result<Batch> {
		Ok(Batch {
			batch,
			msg: self.runtime.batch_message(batch).await?.context("invalid batch id")?,
			task: self.runtime.batch_task(batch).await?.context("invalid batch id")?,
			tx: self.runtime.batch_tx_hash(batch).await?,
		})
	}

	pub async fn message(&self, message: MessageId) -> Result<Message> {
		Ok(Message {
			message,
			recv: self.runtime.message_received_task(message).await?,
			batch: self.runtime.message_batch(message).await?,
			exec: self.runtime.message_executed_task(message).await?,
		})
	}

	pub async fn message_trace(
		&self,
		network: NetworkId,
		message: MessageId,
	) -> Result<MessageTrace> {
		let msg = self.message(message).await?;
		let src = self.sync_status(network).await?;
		let recv = if let Some(recv) = msg.recv { Some(self.task(recv).await?) } else { None };
		let (dest, submit) = if let Some(batch) = msg.batch {
			let batch = self.batch(batch).await?;
			let submit = self.task(batch.task).await?;

			if let Some(Err(err)) = submit.output.clone() {
				anyhow::bail!("Submit task {} failed with error: {}", submit.task, err);
			}

			let dest = self.sync_status(submit.network).await?;
			(Some(dest), Some(submit))
		} else {
			(None, None)
		};
		let exec = if let Some(exec) = msg.exec { Some(self.task(exec).await?) } else { None };
		Ok(MessageTrace {
			message,
			src,
			recv,
			dest,
			submit,
			exec,
		})
	}
}

impl Tc {
	async fn register_network(&self, network: NetworkId) -> Result<Gateway> {
		let connector = self.connector(network)?;
		let config = self.config.network(network)?;
		let contracts = self.config.contracts(network)?;
		let gateway = if let Some(gateway) = self.runtime.network_gateway(network).await? {
			self.set_network_config(network).await?;
			gateway
		} else {
			self.println(None, format!("deploying gateway {network}")).await?;
			let (gateway, block) = connector
				.deploy_gateway(&contracts.additional_params, &contracts.proxy, &contracts.gateway)
				.await?;
			self.println(None, format!("register_network {network}")).await?;
			self.runtime
				.register_network(time_primitives::Network {
					id: network,
					chain_name: ChainName(BoundedVec::truncate_from(config.blockchain.encode())),
					chain_network: ChainNetwork(BoundedVec::truncate_from(config.network.encode())),
					gateway,
					gateway_block: block,
					config: NetworkConfig {
						batch_size: config.batch_size,
						batch_offset: config.batch_offset,
						batch_gas_limit: config.batch_gas_limit,
						shard_task_limit: config.shard_task_limit,
						shard_size: config.shard_size,
						shard_threshold: config.shard_threshold,
					},
				})
				.await?;
			gateway
		};
		Ok(gateway)
	}

	async fn set_network_config(&self, network: NetworkId) -> Result<()> {
		let config = self.config.network(network)?;
		let config = NetworkConfig {
			batch_size: config.batch_size,
			batch_offset: config.batch_offset,
			batch_gas_limit: config.batch_gas_limit,
			shard_task_limit: config.shard_task_limit,
			shard_size: config.shard_size,
			shard_threshold: config.shard_threshold,
		};

		let batch_size = self.runtime.network_batch_size(network).await?;
		let batch_offset = self.runtime.network_batch_offset(network).await?;
		let batch_gas_limit = self.runtime.network_batch_gas_limit(network).await?;
		let shard_task_limit = self.runtime.network_shard_task_limit(network).await?;
		let shard_size = self.runtime.network_shard_size(network).await?;
		let shard_threshold = self.runtime.network_shard_threshold(network).await?;
		if batch_size == config.batch_size
			&& batch_offset == config.batch_offset
			&& batch_gas_limit == config.batch_gas_limit
			&& shard_task_limit == config.shard_task_limit
			&& shard_size == config.shard_size
			&& shard_threshold == config.shard_threshold
		{
			return Ok(());
		}
		self.println(None, format!("set_network_config {network}")).await?;
		self.runtime.set_network_config(network, config).await?;
		Ok(())
	}

	async fn register_routes(&self, gateways: HashMap<NetworkId, Gateway>) -> Result<()> {
		for (src, src_gateway) in gateways.iter().map(|(src, gateway)| (*src, *gateway)) {
			let connector = self.connector(src)?;
			let routes = connector.routes(src_gateway).await?;
			for (dest, dest_gateway) in gateways.iter().map(|(dest, gateway)| (*dest, *gateway)) {
				let config = self.config.network(dest)?;
				let network_prices = self.read_csv_token_prices()?;
				let src_price = gas_price::get_network_price(&network_prices, &src)?;
				let dest_price = get_network_price(&network_prices, &dest)?;
				let ratio = self.calculate_relative_price(src, dest, src_price, dest_price)?;
				let numerator = convert_bigint_to_u128(ratio.numer())?;
				let denominator = convert_bigint_to_u128(ratio.denom())?;
				let route = Route {
					network_id: dest,
					gateway: dest_gateway,
					relative_gas_price: (numerator, denominator),
					gas_limit: config.route_gas_limit,
					base_fee: config.route_base_fee,
				};
				if let Some(r) = routes.iter().find(|r| r.network_id == route.network_id) {
					if r.gas_limit == route.gas_limit
						&& r.base_fee == route.base_fee
						&& r.relative_gas_price() - route.relative_gas_price() < 100_000.0
					{
						continue;
					}
				}
				self.println(None, format!("register_route {src} {dest}")).await?;
				connector.set_route(src_gateway, route).await?;
			}
		}
		Ok(())
	}

	async fn chronicle_config(&self, chronicle_address: &str) -> Result<ChronicleConfig> {
		let config: time_primitives::admin::Config =
			reqwest::get(format!("{chronicle_address}/config")).await?.json().await?;
		Ok(ChronicleConfig {
			network: config.network,
			account: self.parse_address(None, &config.account)?.into(),
			address: self.parse_address(Some(config.network), &config.address)?,
			public_key: config.public_key,
			peer_id: hex::decode(&config.peer_id_hex)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("chronicle returned invalid peer id"))?,
			peer_id_str: config.peer_id,
		})
	}

	async fn chronicle_status(&self, account: &AccountId) -> Result<ChronicleStatus> {
		if !self.runtime.member_registered(account).await? {
			return Ok(ChronicleStatus::Unregistered);
		}
		if !self.runtime.member_online(account).await? {
			return Ok(ChronicleStatus::Registered);
		}
		Ok(ChronicleStatus::Online)
	}

	async fn wait_for_chronicle(&self, chronicle: &str) -> Result<ChronicleConfig> {
		// 20s should be enough since the chronicle waits for
		// 10s to check for a registered network and some margin
		// for the registered network to be finalized.
		for _ in 0..40 {
			match self.chronicle_config(chronicle).await {
				Ok(config) => return Ok(config),
				Err(err) => {
					tracing::info!("waiting for chronicle {chronicle} to come online");
					tracing::debug!("{err:#?}");
					tokio::time::sleep(Duration::from_secs(1)).await
				},
			}
		}
		anyhow::bail!("failed to connect to chronicle");
	}

	pub async fn register_member(
		&self,
		network: NetworkId,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> Result<()> {
		let member = public_key.clone().into_account();
		if self.runtime.member_stake(&member).await? > 0 {
			return Ok(());
		}
		self.println(
			None,
			format!("register_member {}", self.format_address(None, member.clone().into())?),
		)
		.await?;
		let min_stake = self.runtime.min_stake().await?;
		self.runtime.register_member(network, public_key, peer_id, min_stake).await?;
		sleep(Duration::from_secs(20)).await;
		Ok(())
	}

	pub async fn unregister_member(&self, member: AccountId) -> Result<()> {
		if !self.runtime.member_registered(&member).await? {
			return Ok(());
		}
		self.println(
			None,
			format!("unregister_member {}", self.format_address(None, member.clone().into())?),
		)
		.await?;
		self.runtime.unregister_member(member).await?;
		Ok(())
	}

	pub async fn force_shard_offline(&self, shard: ShardId) -> Result<()> {
		if matches!(self.runtime.shard_status(shard).await?, ShardStatus::Offline) {
			return Ok(());
		}
		self.println(None, format!("force_shard_offline {}", shard)).await?;
		self.runtime.force_shard_offline(shard).await?;
		Ok(())
	}

	pub async fn restart_failed_batch(&self, batch_id: BatchId) -> Result<()> {
		self.runtime.restart_failed_batch(batch_id).await?;
		Ok(())
	}
}

impl Tc {
	pub async fn deploy_network(&self, network: NetworkId) -> Result<Gateway> {
		let config = self.config.network(network)?;
		self.faucet(network).await?;
		let gateway = self.register_network(network).await?;
		let gateway_funds = self.parse_balance(Some(network), &config.gateway_funds)?;
		self.fund(Some(network), gateway, gateway_funds, "gateway").await?;
		Ok(gateway)
	}

	pub async fn deploy_chronicle(&self, chronicle: &str) -> Result<()> {
		let chronicle = self.wait_for_chronicle(chronicle).await?;
		let funds = self.parse_balance(None, &self.config.global().chronicle_funds)?;
		self.fund(None, chronicle.account.clone().into(), funds, "chronicle timechain account")
			.await?;
		let config = self.config.network(chronicle.network)?;
		let chronicle_funds =
			self.parse_balance(Some(chronicle.network), &config.chronicle_funds)?;
		self.fund(
			Some(chronicle.network),
			chronicle.address,
			chronicle_funds,
			"chronicle target account",
		)
		.await?;
		self.register_member(chronicle.network, chronicle.public_key, chronicle.peer_id)
			.await?;
		Ok(())
	}

	pub async fn deploy(&self) -> Result<()> {
		let mut gateways = HashMap::new();
		for network in self.connectors.keys().copied() {
			let gateway = self.deploy_network(network).await?;
			gateways.insert(network, gateway);
		}
		self.register_routes(gateways).await?;
		for chronicle in self.config.chronicles() {
			self.deploy_chronicle(chronicle).await?;
		}
		Ok(())
	}

	pub async fn register_shards(&self, network: NetworkId) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		let keys = self.find_online_shard_keys(network).await?;
		let shards = connector.shards(gateway).await?;
		if same(&keys, &shards) {
			return Ok(());
		}
		self.println(None, format!("register_shards {network} {}", keys.len())).await?;
		connector.set_shards(gateway, &keys).await
	}

	pub async fn set_gateway_admin(&self, network: NetworkId, admin: Address) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		if connector.admin(gateway).await? == admin {
			return Ok(());
		}
		self.println(
			None,
			format!("set_gateway_admin {network} {}", self.format_address(Some(network), admin)?),
		)
		.await?;
		connector.set_admin(gateway, admin).await?;
		Ok(())
	}

	pub async fn redeploy_gateway(&self, network: NetworkId) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		let contracts = self.config.contracts(network)?;
		self.println(None, format!("redeploying gateway {network}")).await?;
		connector
			.redeploy_gateway(&contracts.additional_params, gateway, &contracts.gateway)
			.await?;
		Ok(())
	}

	pub async fn deploy_tester(&self, network: NetworkId) -> Result<(Address, u64)> {
		let contracts = self.config.contracts(network)?;
		let (connector, gateway) = self.gateway(network).await?;
		self.println(None, format!("deploy tester {network}")).await?;
		connector.deploy_test(gateway, &contracts.tester).await
	}

	pub async fn estimate_message_gas_limit(
		&self,
		dest_network: NetworkId,
		dest_addr: Address,
		src_network: NetworkId,
		src_addr: Address,
		payload: Vec<u8>,
	) -> Result<u128> {
		let connector = self.connector(dest_network)?;
		connector
			.estimate_message_gas_limit(dest_addr, src_network, src_addr, payload)
			.await
	}

	pub async fn estimate_message_cost(
		&self,
		src_network: NetworkId,
		dest_network: NetworkId,
		gas_limit: u128,
		payload: Vec<u8>,
	) -> Result<u128> {
		let (connector, gateway) = self.gateway(src_network).await?;
		connector.estimate_message_cost(gateway, dest_network, gas_limit, payload).await
	}

	#[allow(clippy::too_many_arguments)]
	pub async fn send_message(
		&self,
		src_network: NetworkId,
		src_addr: Address,
		dest_network: NetworkId,
		dest_addr: Address,
		gas_limit: u128,
		gas_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		let connector = self.connector(src_network)?;
		let id = self
			.println(
				None,
				format!(
					"send message to {} {} with {} gas for {}",
					dest_network,
					self.format_address(Some(dest_network), dest_addr)?,
					gas_limit,
					self.format_balance(Some(src_network), gas_cost)?,
				),
			)
			.await?;
		let msg_id = connector
			.send_message(src_addr, dest_network, dest_addr, gas_limit, gas_cost, payload)
			.await?;
		self.println(
			Some(id),
			format!(
				"sent message {} to {} {} with {} gas for {}",
				hex::encode(msg_id),
				dest_network,
				self.format_address(Some(dest_network), dest_addr)?,
				gas_limit,
				self.format_balance(Some(src_network), gas_cost)?,
			),
		)
		.await?;
		Ok(msg_id)
	}

	pub async fn remove_task(&self, task_id: TaskId) -> Result<()> {
		self.runtime.remove_task(task_id).await
	}

	pub async fn complete_batch(&self, network_id: NetworkId, batch_id: BatchId) -> Result<()> {
		let gmp_event = GmpEvent::BatchExecuted { batch_id, tx_hash: None };
		let events = GmpEvents(BoundedVec::truncate_from(vec![gmp_event]));
		self.runtime.submit_gmp_events(network_id, events).await
	}

	pub async fn withdraw_funds(
		&self,
		network: NetworkId,
		amount: u128,
		address: Address,
	) -> Result<()> {
		let (connector, gateway) = self.gateway(network).await?;
		self.println(
			None,
			format!(
				"withdrawing funds {network} {} to {}",
				self.format_balance(Some(network), amount)?,
				self.format_address(Some(network), address)?,
			),
		)
		.await?;
		connector.withdraw_funds(gateway, amount, address).await
	}

	pub async fn setup_test(&self, src: NetworkId, dest: NetworkId) -> Result<(Address, Address)> {
		// networks
		self.deploy().await?;
		let (src_addr, src_block) = self.deploy_tester(src).await?;
		let (dest_addr, dest_block) = self.deploy_tester(dest).await?;
		tracing::info!("deployed at src block {}, dest block {}", src_block, dest_block);
		/*let networks = self.networks().await?;
		self.print_table(None, "networks", networks.clone()).await?;
		for network in networks {
			let routes = self.routes(network.network).await?;
			self.print_table(None, "routes", routes).await?;
		}*/
		// chronicles
		let mut blocks = self.finality_notification_stream();
		let mut id = None;
		while blocks.next().await.is_some() {
			let chronicles = self.chronicles().await?;
			let not_registered = chronicles.iter().any(|c| c.status != ChronicleStatus::Online);
			tracing::info!("waiting for chronicles to be registered");
			id = Some(self.print_table(id, "chronicles", chronicles).await?);
			if !not_registered {
				break;
			}
		}
		// shards
		let mut id = None;
		while blocks.next().await.is_some() {
			let src_keys = self.find_online_shard_keys(src).await?;
			let dest_keys = self.find_online_shard_keys(dest).await?;
			if !src_keys.is_empty() && !dest_keys.is_empty() {
				break;
			}
			tracing::info!("waiting for shards to come online");
			let shards = self.shards().await?;
			id = Some(self.print_table(id, "shards", shards).await?);
		}
		// registered shards
		self.register_shards(src).await?;
		self.register_shards(dest).await?;
		while blocks.next().await.is_some() {
			let shards = self.shards().await?;
			let is_registered =
				shards.iter().any(|shard| shard.registered && shard.network == dest);
			tracing::info!("waiting for shard to be registered");
			id = Some(self.print_table(id, "shards", shards).await?);
			if is_registered {
				break;
			}
		}
		Ok((src_addr, dest_addr))
	}

	pub async fn wait_for_sync(&self, network: NetworkId) -> Result<()> {
		let mut blocks = self.finality_notification_stream();
		while blocks.next().await.is_some() {
			let status = self.sync_status(network).await?;
			tracing::info!(
				"waiting for network {network} to sync {} / {}",
				status.sync,
				status.block
			);
			if status.next_sync > status.block {
				break;
			}
		}
		Ok(())
	}

	pub async fn print_table<R: IntoRow>(
		&self,
		id: Option<TableRef>,
		title: &str,
		table: Vec<R>,
	) -> Result<TableRef> {
		let mut out = Vec::new();
		{
			let mut wtr = csv::Writer::from_writer(&mut out);
			for row in table {
				wtr.serialize(row.into_row(self)?)?;
			}
			wtr.flush()?;
		}
		self.msg.csv(id, title, out).await
	}

	pub async fn println(&self, id: Option<TextRef>, line: impl Into<String>) -> Result<TextRef> {
		self.msg.text(id, line.into()).await
	}

	pub async fn log(&self, query: Query, since: String) -> Result<TableRef> {
		let logs = loki::logs(query, since).await?;
		self.print_table(None, "logs", logs).await
	}

	pub async fn debug_transaction(&self, network: NetworkId, hash: Hash) -> Result<String> {
		let connector = self.connector(network)?;
		connector.debug_transaction(hash).await
	}

	pub async fn dump_state(&self, network: NetworkId) -> Result<String> {
		let connector = self.connector(network)?;
		connector.dump_state().await
	}

	pub async fn load_state(&self, network: NetworkId, state: String) -> Result<()> {
		let connector = self.connector(network)?;
		connector.load_state(state).await
	}
}
