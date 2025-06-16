use crate::table::IntoRow;
use anyhow::{Context, Result};
use futures::stream::{BoxStream, FuturesUnordered, StreamExt};
use futures::TryStreamExt;
use polkadot_sdk::sp_runtime::BoundedVec;
use scale_codec::{Decode, Encode};
use std::cell::OnceCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tc_subxt::SubxtClient;
use time_primitives::{
	AccountId, Address32, BatchGasParams, BatchId, BlockHash, BlockNumber, ChainName, Currency,
	GatewayMessage, GmpEvent, GmpEvents, GmpMessage, Hash, IChain, IConnect, IConnectorAdmin,
	MemberStatus, MessageId, NetworkConfig, PeerId, Route, ShardId, ShardStatus, TaskId,
	TssPublicKey,
};

mod benchmark;
pub mod config;
mod env;
mod gas_price;
mod loki;
mod print;
mod table;

pub use crate::benchmark::{Benchmark, BenchmarkStats};
pub use crate::config::Config;
pub use crate::env::Mnemonics;
pub use crate::loki::{Log, Query};
pub use crate::print::{Sender, TableRef, TextRef};
pub use gmp::Backend;
pub use time_primitives::NetworkId;

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
	chains: HashMap<NetworkId, Arc<dyn IConnect>>,
	connectors: HashMap<NetworkId, OnceCell<Arc<dyn IConnectorAdmin>>>,
	msg: Sender,
}

impl Tc {
	pub async fn from_env(env: PathBuf, config: &str, msg: Sender, tx_db: PathBuf) -> Result<Self> {
		dotenv::from_path(env.join(".env")).ok();
		let config = Config::from_env(env, config)?;
		let env = Mnemonics::from_env();
		Self::new(config, env, msg, tx_db).await
	}

	pub async fn new(config: Config, env: Mnemonics, msg: Sender, tx_db: PathBuf) -> Result<Self> {
		let timechain_url = config.global().timechain_url.clone();
		let runtime = tokio::task::spawn(async move {
			while let Err(err) = SubxtClient::get_client(&timechain_url).await {
				tracing::info!("waiting for timechain to start: {err:?}");
				sleep_or_abort(Duration::from_secs(10)).await?;
			}
			let runtime = SubxtClient::with_key(&timechain_url, &env.timechain_mnemonic, &tx_db)
				.await
				.context("failed to connect to timechain")?;
			Ok::<_, anyhow::Error>(runtime)
		});
		let mut chains = HashMap::new();
		let mut connectors = HashMap::new();
		{
			for (id, network) in config.networks() {
				chains.insert(*id, network.backend.chain(*id, &env.target_mnemonic)?);
				connectors.insert(*id, Default::default());
			}
		}
		let runtime = runtime.await??;
		Ok(Self {
			config,
			runtime,
			chains,
			connectors,
			msg,
		})
	}

	fn chain(&self, network: NetworkId) -> Result<&dyn IChain> {
		Ok(self
			.chains
			.get(&network)
			.with_context(|| format!("no connector configured for {network}"))?
			.chain())
	}

	async fn connector(&self, network: NetworkId) -> Result<&dyn IConnectorAdmin> {
		let cell = self
			.connectors
			.get(&network)
			.with_context(|| format!("no connector configured for {network}"))?;
		if let Some(connector) = cell.get() {
			return Ok(&**connector);
		}
		let config = self.config.network(network)?;
		let connector = self
			.chains
			.get(&network)
			.with_context(|| format!("no connector configured for {network}"))?
			.connect_admin(config.url.clone())
			.await?;
		cell.set(connector).ok();
		Ok(&**cell.get().expect("once cel failed"))
	}

	pub async fn gateway(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<(&dyn IConnectorAdmin, Address32)> {
		let connector = self.connector(network).await?;
		let gateway = self
			.runtime
			.network_gateway(network, block_hash)
			.await?
			.with_context(|| format!("no gateway configured for {network}"))?;
		Ok((connector, gateway))
	}

	pub fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.runtime.finality_notification_stream()
	}

	pub async fn latest_block(&self) -> Result<(BlockHash, BlockNumber)> {
		self.runtime.latest_block().await
	}

	pub async fn runtime_upgrade(&self, path: &Path) -> Result<()> {
		self.println(None, "runtime-upgrade").await?;
		let bytecode = std::fs::read(path)?;
		self.runtime.set_code(bytecode).await
	}

	pub async fn find_online_shard_keys(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<Vec<TssPublicKey>> {
		let mut shards = vec![];
		for shard_id in self.runtime.shards(block_hash).await? {
			match self.runtime.shard_network(shard_id, block_hash).await {
				Ok(Some(shard_network)) if shard_network == network => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			};
			match self.runtime.shard_status(shard_id, block_hash).await {
				Ok(ShardStatus::Online) => {},
				Ok(_) => continue,
				Err(err) => {
					tracing::info!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			}
			let shard_key = match self.runtime.shard_public_key(shard_id, block_hash).await {
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

	pub fn parse_address(&self, network: Option<NetworkId>, address: &str) -> Result<Address32> {
		if let Some(network) = network {
			self.chain(network)?.parse_address(address)
		} else {
			let address: AccountId = address
				.parse()
				.map_err(|err| anyhow::anyhow!("invalid timechain account: {err}"))?;
			Ok(address.into())
		}
	}

	pub fn format_address(&self, network: Option<NetworkId>, address: Address32) -> Result<String> {
		if let Some(network) = network {
			Ok(self.chain(network)?.format_address(address))
		} else {
			let address: AccountId = address.into();
			Ok(time_primitives::format_address(&address))
		}
	}

	pub fn currency(&self, network: Option<NetworkId>) -> Result<Currency> {
		if let Some(network) = network {
			let config = self.config.network(network)?;
			Ok(Currency::new(config.currency_decimals, config.currency_symbol.clone()))
		} else {
			Ok(Currency::new(12, "ANLG".into()))
		}
	}

	pub fn parse_balance(&self, network: Option<NetworkId>, balance: &str) -> Result<u128> {
		self.currency(network)?.parse(balance)
	}

	pub fn format_balance(&self, network: Option<NetworkId>, balance: u128) -> Result<String> {
		Ok(self.currency(network)?.format(balance))
	}

	pub fn address(&self, network: Option<NetworkId>) -> Result<Address32> {
		Ok(if let Some(network) = network {
			self.chain(network)?.address()
		} else {
			self.runtime.account_id().clone().into()
		})
	}

	pub async fn faucet(&self, network: NetworkId, block_hash: BlockHash) -> Result<()> {
		let config = self.config.network(network)?;
		let Some(admin_funds) = config.admin_funds.as_ref() else {
			return Ok(());
		};
		let admin_funds = self.parse_balance(Some(network), admin_funds)?;
		let current_admin_funds =
			self.balance(Some(network), self.address(Some(network))?, block_hash).await?;
		let faucet = admin_funds.saturating_sub(current_admin_funds);
		if faucet == 0 {
			return Ok(());
		}
		self.println(
			None,
			format!("faucet {network} {}", self.format_balance(Some(network), faucet)?),
		)
		.await?;
		self.connector(network).await?.faucet(faucet).await.with_context(|| {
			format!(
				"faucet failed or is unsupported, please transfer {} to {}",
				self.format_balance(Some(network), faucet).unwrap(),
				self.format_address(Some(network), self.address(Some(network)).unwrap())
					.unwrap(),
			)
		})
	}

	pub async fn balance(
		&self,
		network: Option<NetworkId>,
		address: Address32,
		block_hash: BlockHash,
	) -> Result<u128> {
		if let Some(network) = network {
			self.connector(network).await?.balance(address).await
		} else {
			self.runtime.balance(&address.into(), block_hash).await
		}
	}

	pub async fn transfer(
		&self,
		network: Option<NetworkId>,
		address: Address32,
		balance: u128,
	) -> Result<()> {
		self.println(
			None,
			format!(
				"transferring {} to {}",
				self.format_balance(network, balance)?,
				self.format_address(network, address)?,
			),
		)
		.await?;
		if let Some(network) = network {
			self.connector(network).await?.transfer(address, balance).await?;
		} else {
			self.runtime.transfer(address.into(), balance).await?;
		}
		Ok(())
	}

	pub async fn fund(
		&self,
		network: Option<NetworkId>,
		address: Address32,
		min_balance: u128,
		label: &str,
		block_hash: BlockHash,
	) -> Result<()> {
		let balance = self.balance(network, address, block_hash).await?;
		let diff = min_balance.saturating_sub(balance);
		if diff > 0 {
			self.println(None, format!("funding {label}")).await?;
			self.transfer(network, address, diff).await?;
		}
		Ok(())
	}

	pub fn iter(&self) -> impl Iterator<Item = NetworkId> + '_ {
		self.chains.keys().copied()
	}
}

#[derive(Clone, Debug)]
pub struct Network {
	pub network: NetworkId,
	pub chain_name: String,
	pub info: Option<NetworkInfo>,
}

#[derive(Clone, Debug)]
pub struct NetworkInfo {
	pub gateway: Address32,
	pub gateway_balance: u128,
	pub admin: Address32,
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
	pub target_address: Address32,
	pub target_balance: u128,
}

struct ChronicleConfig {
	network: NetworkId,
	account: AccountId,
	peer_id: PeerId,
	peer_id_str: String,
	address: Address32,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShardRegistration {
	Unknown,
	Unregistered,
	RegisteredGateway,
	Registered,
}

impl std::fmt::Display for ShardRegistration {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let label = match self {
			Self::Unknown => "unknown",
			Self::Unregistered => "unregistered",
			Self::RegisteredGateway => "gateway",
			Self::Registered => "registered",
		};
		f.write_str(label)
	}
}

#[derive(Clone, Debug)]
pub struct Shard {
	pub shard: ShardId,
	pub network: NetworkId,
	pub status: ShardStatus,
	pub key: Option<TssPublicKey>,
	pub registered: ShardRegistration,
	pub size: u16,
	pub threshold: u16,
	pub assigned: usize,
	pub batch_register: Option<BatchId>,
	pub batch_unregister: Option<BatchId>,
}

#[derive(Clone, Debug)]
pub struct Member {
	pub account: AccountId,
	pub status: MemberStatus,
}

#[derive(Clone, Debug)]
pub struct Task {
	pub task: TaskId,
	pub network: NetworkId,
	pub descriptor: time_primitives::Task,
	pub output: Option<Result<(), String>>,
	pub shard: Option<ShardId>,
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

impl Tc {
	pub async fn read_events_blocks(
		&self,
		task: TaskId,
		block_hash: BlockHash,
	) -> Result<Range<u64>> {
		let task = self.runtime.task(task, block_hash).await?.context("no read event task")?;
		let time_primitives::Task::ReadGatewayEvents { blocks } = task else {
			anyhow::bail!("invalid read event task descriptor");
		};
		Ok(blocks)
	}

	pub async fn sync_status(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<SyncStatus> {
		let sync_task = self
			.runtime
			.read_events_task(network, block_hash)
			.await?
			.context("no read events task")?;
		let blocks = self.read_events_blocks(sync_task, block_hash).await?;
		let block = self
			.connector(network)
			.await?
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

	pub async fn networks(&self, block_hash: BlockHash) -> Result<Vec<Network>> {
		let network_ids = self.runtime.networks(block_hash).await?;
		let mut networks = vec![];
		for network in network_ids {
			let chain_name = self
				.runtime
				.network_name(network, block_hash)
				.await?
				.context("invalid network")?;
			let chain_name =
				String::decode(&mut chain_name.0.to_vec().as_slice()).unwrap_or_default();
			let info = match self.gateway(network, block_hash).await {
				Ok((connector, gateway)) => {
					let gateway_balance = connector.balance(gateway).await?;
					let admin = connector.admin(gateway).await?;
					let admin_balance = connector.balance(admin).await?;
					let sync_status = self.sync_status(network, block_hash).await?;
					let unassigned_tasks =
						self.runtime.unassigned_tasks(network, block_hash).await?;
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
			networks.push(Network { network, chain_name, info });
		}
		Ok(networks)
	}

	pub async fn chronicles(&self, block_hash: BlockHash) -> Result<Vec<Chronicle>> {
		let mut chronicles = vec![];
		for chronicle in self.config.chronicles() {
			let config = self.chronicle_config(chronicle).await?;
			let status = self.chronicle_status(&config.account, block_hash).await?;
			let network = config.network;
			let balance = self.balance(None, config.account.clone().into(), block_hash).await?;
			let target_balance = self.balance(Some(network), config.address, block_hash).await?;
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

	async fn registered_shards(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<Vec<TssPublicKey>> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
		connector.shards(gateway).await
	}

	pub async fn shards(&self, block_hash: BlockHash) -> Result<Vec<Shard>> {
		let mut shards = vec![];
		let mut registered_shards = HashMap::new();
		for shard in self.runtime.shards(block_hash).await? {
			let Some(network) = self.runtime.shard_network(shard, block_hash).await? else {
				continue;
			};
			if let Entry::Vacant(e) = registered_shards.entry(network) {
				match self.registered_shards(network, block_hash).await {
					Ok(shards) => {
						e.insert(shards);
					},
					Err(err) => {
						tracing::error!("{err}");
					},
				};
			}
			let status = self.runtime.shard_status(shard, block_hash).await?;
			let key = self.runtime.shard_commitment(shard, block_hash).await?.map(|c| c.0[0]);
			let size = self.runtime.shard_members(shard, block_hash).await?.len() as u16;
			let threshold = self.runtime.shard_threshold(shard, block_hash).await?;
			let mut registered = ShardRegistration::Unknown;
			let mut batch_register = None;
			let mut batch_unregister = None;
			if let Some(key) = key {
				if let Some(shards) = registered_shards.get(&network) {
					registered = ShardRegistration::Unregistered;
					if shards.contains(&key) {
						registered = ShardRegistration::RegisteredGateway;
						if self.runtime.is_shard_registered(key, block_hash).await? {
							registered = ShardRegistration::Registered;
						}
					}
				}
				batch_register = self.runtime.shard_register_batch(key, block_hash).await?;
				batch_unregister = self.runtime.shard_unregister_batch(key, block_hash).await?;
			}
			let assigned = self.runtime.assigned_tasks(shard, block_hash).await?.len();
			shards.push(Shard {
				shard,
				network,
				status,
				key,
				registered,
				size,
				threshold,
				assigned,
				batch_register,
				batch_unregister,
			});
		}
		Ok(shards)
	}

	pub async fn unassigned_tasks(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<Vec<Task>> {
		let task_ids = self.runtime.unassigned_tasks(network, block_hash).await?;
		let mut tasks = Vec::with_capacity(task_ids.len());
		for id in task_ids {
			tasks.push(self.task(id, block_hash).await?);
		}
		Ok(tasks)
	}

	pub async fn assigned_tasks(&self, shard: ShardId, block_hash: BlockHash) -> Result<Vec<Task>> {
		let task_ids = self.runtime.assigned_tasks(shard, block_hash).await?;
		let mut tasks = Vec::with_capacity(task_ids.len());
		for id in task_ids {
			tasks.push(self.task(id, block_hash).await?);
		}
		Ok(tasks)
	}

	pub async fn failed_batches(&self, block_hash: BlockHash) -> Result<Vec<Batch>> {
		let batch_ids = self.runtime.failed_batches(block_hash).await?;
		let mut batches = Vec::with_capacity(batch_ids.len());
		for id in batch_ids {
			batches.push(self.batch(id, block_hash).await?);
		}
		Ok(batches)
	}

	pub async fn pending_batches(&self, block_hash: BlockHash) -> Result<Vec<Batch>> {
		let batch_ids = self.runtime.pending_batches(block_hash).await?;
		let mut batches = Vec::with_capacity(batch_ids.len());
		for id in batch_ids {
			batches.push(self.batch(id, block_hash).await?);
		}
		Ok(batches)
	}

	pub async fn members(&self, shard: ShardId, block_hash: BlockHash) -> Result<Vec<Member>> {
		let shard_members = self.runtime.shard_members(shard, block_hash).await?;
		let mut members = Vec::with_capacity(shard_members.len());
		for (account, status) in shard_members {
			members.push(Member { account, status })
		}
		Ok(members)
	}

	pub async fn routes(&self, network: NetworkId, block_hash: BlockHash) -> Result<Vec<Route>> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
		connector.routes(gateway).await
	}

	pub async fn events(
		&self,
		network: NetworkId,
		blocks: Range<u64>,
		block_hash: BlockHash,
	) -> Result<Vec<GmpEvent>> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
		connector.read_events(gateway, blocks).await
	}

	pub async fn messages(
		&self,
		network: NetworkId,
		tester: Address32,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		let connector = self.connector(network).await?;
		connector.recv_messages(tester, blocks).await
	}

	pub async fn task(&self, task: TaskId, block_hash: BlockHash) -> Result<Task> {
		Ok(Task {
			task,
			network: self
				.runtime
				.task_network(task, block_hash)
				.await?
				.context("invalid task id")?,
			descriptor: self.runtime.task(task, block_hash).await?.context("invalid task id")?,
			output: self.runtime.task_output(task, block_hash).await?.map(|o| {
				o.map_err(|e| String::decode(&mut e.0.to_vec().as_slice()).unwrap_or_default())
			}),
			shard: self.runtime.assigned_shard(task, block_hash).await?,
		})
	}

	pub async fn max_fee_per_gas(&self, network: NetworkId) -> Result<u128> {
		let connector = self.connector(network).await?;
		let fee = connector.max_fee_per_gas().await?;
		Ok(fee)
	}

	pub async fn block_gas_limit(&self, network: NetworkId) -> Result<u64> {
		let connector = self.connector(network).await?;
		let gas_limit = connector.block_gas_limit().await?;
		Ok(gas_limit)
	}

	pub async fn batch(&self, batch: BatchId, block_hash: BlockHash) -> Result<Batch> {
		Ok(Batch {
			batch,
			msg: self
				.runtime
				.batch_message(batch, block_hash)
				.await?
				.context("invalid batch id")?,
			task: self.runtime.batch_task(batch, block_hash).await?.context("invalid batch id")?,
			tx: self.runtime.batch_tx_hash(batch, block_hash).await?,
		})
	}

	pub async fn message(&self, message: MessageId, block_hash: BlockHash) -> Result<Message> {
		Ok(Message {
			message,
			recv: self.runtime.message_received_task(message, block_hash).await?,
			batch: self.runtime.message_batch(message, block_hash).await?,
			exec: self.runtime.message_executed_task(message, block_hash).await?,
		})
	}

	pub async fn is_message_executed(
		&self,
		message: MessageId,
		block_hash: BlockHash,
	) -> Result<bool> {
		Ok(self.runtime.message_executed_task(message, block_hash).await?.is_some())
	}

	pub async fn is_task_executed(&self, task: TaskId, block_hash: BlockHash) -> Result<bool> {
		Ok(self.runtime.task_output(task, block_hash).await?.is_some())
	}

	pub async fn is_batch_executed(&self, batch: BatchId, block_hash: BlockHash) -> Result<bool> {
		Ok(self.runtime.batch_tx_hash(batch, block_hash).await?.is_some())
	}

	pub async fn message_trace(
		&self,
		network: NetworkId,
		message: MessageId,
		block_hash: BlockHash,
	) -> Result<MessageTrace> {
		let msg = self.message(message, block_hash).await?;
		let src = self.sync_status(network, block_hash).await?;
		let recv =
			if let Some(recv) = msg.recv { Some(self.task(recv, block_hash).await?) } else { None };
		let (dest, submit) = if let Some(batch) = msg.batch {
			let batch = self.batch(batch, block_hash).await?;
			let submit = self.task(batch.task, block_hash).await?;

			if let Some(Err(err)) = submit.output.clone() {
				tracing::error!("Submit task {} failed with error: {}", submit.task, err);
			}

			let dest = self.sync_status(submit.network, block_hash).await?;
			(Some(dest), Some(submit))
		} else {
			(None, None)
		};
		let exec =
			if let Some(exec) = msg.exec { Some(self.task(exec, block_hash).await?) } else { None };
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
	fn network_config(&self, network: NetworkId) -> Result<NetworkConfig> {
		let config = self.config.network(network)?;
		Ok(NetworkConfig {
			batch_size: config.batch_size,
			batch_offset: config.batch_offset,
			shard_task_limit: config.shard_task_limit,
			shard_size: config.shard_size,
			shard_threshold: config.shard_threshold,
			batch_gas_params: BatchGasParams {
				batch_gas_limit: config.batch_gas_limit,
				batch_exec_gas: config.batch_exec_gas,
				reg_op_exec_gas: config.reg_op_exec_gas,
				unreg_op_exec_gas: config.unreg_op_exec_gas,
				msg_op_exec_gas: config.msg_op_exec_gas,
				msg_byte_gas: config.msg_byte_gas,
			},
			max_gas_price: config.max_gas_price,
		})
	}

	async fn register_network(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<Address32> {
		let connector = self.connector(network).await?;
		let config = self.config.network(network)?;
		let backend = self.config.backend(network)?;
		let gateway =
			if let Some(gateway) = self.runtime.network_gateway(network, block_hash).await? {
				self.set_network_config(network, block_hash).await?;
				gateway
			} else {
				self.println(None, format!("deploying gateway {network}")).await?;
				let (gateway, block) =
					connector.deploy_gateway(&backend.proxy, &backend.gateway).await?;
				self.println(None, format!("register_network {network}")).await?;
				self.runtime
					.register_network(time_primitives::Network {
						id: network,
						chain_name: ChainName(BoundedVec::truncate_from(config.name.encode())),
						gateway,
						gateway_block: block,
						config: self.network_config(network)?,
					})
					.await?;
				gateway
			};
		Ok(gateway)
	}

	pub async fn set_network_config(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<()> {
		let config = self.network_config(network)?;
		let old_config = self.runtime.network_config(network, block_hash).await?;
		if config == old_config {
			return Ok(());
		}
		self.println(None, format!("set_network_config {network}")).await?;
		self.runtime.set_network_config(network, config).await?;
		Ok(())
	}

	pub async fn register_routes(&self, gateways: HashMap<NetworkId, Address32>) -> Result<()> {
		let mut set_routes = FuturesUnordered::new();
		for (src, src_gateway) in gateways.iter().map(|(src, gateway)| (*src, *gateway)) {
			let connector = self.connector(src).await?;
			let routes = connector.routes(src_gateway).await?;
			for (dest, dest_gateway) in gateways.iter().map(|(dest, gateway)| (*dest, *gateway)) {
				if src == dest {
					continue;
				}
				let config = self.config.network(dest)?;
				let gas_price = self.config.gas_price(src, dest)?;
				let route = Route {
					network_id: dest,
					gateway: dest_gateway,
					max_gas_limit: config.route_max_gas_limit,
					msg_gas: config.msg_gas(),
					msg_byte_gas: config.msg_byte_gas(),
					gas_price,
					msg_fee: config.route_msg_fee,
				};
				if routes.contains(&route) {
					continue;
				}
				self.println(None, format!("register_route {src} {dest}")).await?;
				set_routes.push(connector.set_route(src_gateway, route));
			}
		}
		while let Some(result) = set_routes.next().await {
			result?;
		}
		Ok(())
	}

	pub async fn register_all_routes(&self, block_hash: BlockHash) -> Result<()> {
		let gateways = FuturesUnordered::new();
		for network in self.iter() {
			let fut = self.gateway(network, block_hash);
			gateways.push(async move {
				let (_, gateway) = fut.await?;
				Ok::<_, anyhow::Error>((network, gateway))
			});
		}

		let routes: HashMap<NetworkId, Address32> = gateways.try_collect().await?;
		self.register_routes(routes).await
	}

	async fn chronicle_config(&self, chronicle_address: &str) -> Result<ChronicleConfig> {
		let config: time_primitives::admin::Config =
			reqwest::get(format!("{chronicle_address}/config")).await?.json().await?;
		Ok(ChronicleConfig {
			network: config.network,
			account: self.parse_address(None, &config.account)?.into(),
			address: self.parse_address(Some(config.network), &config.address)?,
			peer_id: hex::decode(&config.peer_id_hex)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("chronicle returned invalid peer id"))?,
			peer_id_str: config.peer_id,
		})
	}

	async fn chronicle_status(
		&self,
		account: &AccountId,
		block_hash: BlockHash,
	) -> Result<ChronicleStatus> {
		if !self.runtime.member_registered(account, block_hash).await? {
			return Ok(ChronicleStatus::Unregistered);
		}
		if !self.runtime.member_online(account, block_hash).await? {
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
		account: AccountId,
		peer_id: PeerId,
		block_hash: BlockHash,
	) -> Result<()> {
		if self.runtime.member_registered(&account, block_hash).await? {
			return Ok(());
		}
		self.println(
			None,
			format!("register_member {}", self.format_address(None, account.clone().into())?),
		)
		.await?;
		self.runtime.register_member(network, account, peer_id).await?;
		Ok(())
	}

	pub async fn unregister_member(&self, member: AccountId, block_hash: BlockHash) -> Result<()> {
		if !self.runtime.member_registered(&member, block_hash).await? {
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

	pub async fn force_shard_offline(&self, shard: ShardId, block_hash: BlockHash) -> Result<()> {
		if matches!(self.runtime.shard_status(shard, block_hash).await?, ShardStatus::Offline) {
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
	pub async fn deploy_network(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<Address32> {
		let config = self.config.network(network)?;
		self.faucet(network, block_hash).await?;
		let gateway = self.register_network(network, block_hash).await?;
		let gateway_funds = self.parse_balance(Some(network), &config.gateway_funds)?;
		self.fund(Some(network), gateway, gateway_funds, "gateway", block_hash).await?;
		Ok(gateway)
	}

	pub async fn deploy_chronicle(&self, chronicle: &str, block_hash: BlockHash) -> Result<()> {
		let chronicle = self.wait_for_chronicle(chronicle).await?;
		let funds = self.parse_balance(None, &self.config.global().chronicle_funds)?;
		let fund_tc = self.fund(
			None,
			chronicle.account.clone().into(),
			funds,
			"chronicle timechain account",
			block_hash,
		);
		let config = self.config.network(chronicle.network)?;
		let chronicle_funds =
			self.parse_balance(Some(chronicle.network), &config.chronicle_funds)?;
		let fund_target = self.fund(
			Some(chronicle.network),
			chronicle.address,
			chronicle_funds,
			"chronicle target account",
			block_hash,
		);
		let (result_tc, result_target) = futures::future::join(fund_tc, fund_target).await;
		result_tc?;
		result_target?;
		self.register_member(chronicle.network, chronicle.account, chronicle.peer_id, block_hash)
			.await?;
		Ok(())
	}

	pub async fn deploy(&self, block_hash: BlockHash) -> Result<()> {
		let mut deploy_network = FuturesUnordered::new();
		for network in self.iter() {
			deploy_network.push(async move {
				let gateway = self.deploy_network(network, block_hash).await?;
				Ok::<_, anyhow::Error>((network, gateway))
			});
		}
		let mut gateways = HashMap::new();
		while let Some(result) = deploy_network.next().await {
			let (network, gateway) = result?;
			gateways.insert(network, gateway);
		}

		self.register_routes(gateways).await?;

		let mut deploy_chronicle = FuturesUnordered::new();
		for chronicle in self.config.chronicles() {
			deploy_chronicle.push(self.deploy_chronicle(chronicle, block_hash));
		}
		while let Some(result) = deploy_chronicle.next().await {
			result?;
		}
		Ok(())
	}

	pub async fn register_online_shards(&self, block_hash: BlockHash) -> Result<()> {
		let mut register_shards = FuturesUnordered::new();
		for network in self.iter() {
			let keys = self.find_online_shard_keys(network, block_hash).await?;
			register_shards.push(self.register_shards(network, keys, block_hash));
		}
		while let Some(result) = register_shards.next().await {
			result?;
		}
		Ok(())
	}

	pub async fn register_shards(
		&self,
		network: NetworkId,
		keys: Vec<TssPublicKey>,
		block_hash: BlockHash,
	) -> Result<()> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
		let shards = connector.shards(gateway).await?;
		let mut register = Vec::with_capacity(keys.len());
		let mut revoke = Vec::with_capacity(shards.len());
		let num_sessions = self.config.network(network)?.num_sessions();
		for key in &keys {
			if !shards.contains(key) {
				register.push((*key, num_sessions));
			}
		}
		for key in &shards {
			if !keys.contains(key) {
				revoke.push((*key, num_sessions));
			}
		}
		if register.is_empty() && revoke.is_empty() {
			return Ok(());
		}
		self.println(
			None,
			format!("register_shards {network} {} {}", register.len(), revoke.len()),
		)
		.await?;
		connector.set_shards(gateway, &register, &revoke).await
	}

	pub async fn set_gateway_admin(
		&self,
		network: NetworkId,
		admin: Address32,
		block_hash: BlockHash,
	) -> Result<()> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
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

	pub async fn redeploy_gateway(&self, network: NetworkId, block_hash: BlockHash) -> Result<()> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
		let backend = self.config.backend(network)?;
		self.println(None, format!("redeploying gateway {network}")).await?;
		connector.redeploy_gateway(gateway, &backend.gateway).await?;
		Ok(())
	}

	pub async fn deploy_tester(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<(Address32, u64)> {
		let backend = self.config.backend(network)?;
		let (connector, gateway) = self.gateway(network, block_hash).await?;
		let id = self.println(None, format!("deploy tester {network}")).await?;
		let tester = connector.deploy_tester(gateway, &backend.tester).await?;
		self.println(
			Some(id),
			format!(
				"deployed tester {} at {}",
				self.format_address(Some(network), tester.0)?,
				tester.1,
			),
		)
		.await?;
		Ok(tester)
	}

	pub async fn estimate_message_gas_limit(
		&self,
		dest_network: NetworkId,
		dest_addr: Address32,
		src_network: NetworkId,
		src_addr: Address32,
		payload: Vec<u8>,
	) -> Result<u64> {
		let connector = self.connector(dest_network).await?;
		connector
			.estimate_message_gas_limit(dest_addr, src_network, src_addr, payload)
			.await
	}

	pub async fn estimate_message_cost(
		&self,
		src_network: NetworkId,
		dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
		block_hash: BlockHash,
	) -> Result<u128> {
		let (connector, gateway) = self.gateway(src_network, block_hash).await?;
		connector
			.estimate_message_cost(gateway, dest_network, msg_size, gas_limit)
			.await
	}

	#[allow(clippy::too_many_arguments)]
	pub async fn send_message(
		&self,
		src_network: NetworkId,
		src_addr: Address32,
		dest_network: NetworkId,
		dest_addr: Address32,
		gas_limit: u64,
		gas_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		let connector = self.connector(src_network).await?;
		let id = self
			.println(
				None,
				format!(
					"send message from {} {} to {} {} with {} gas for {} {}$",
					src_network,
					self.format_address(Some(src_network), src_addr)?,
					dest_network,
					self.format_address(Some(dest_network), dest_addr)?,
					gas_limit,
					self.format_balance(Some(src_network), gas_cost)?,
					self.config.balance_to_usd(src_network, gas_cost)?,
				),
			)
			.await?;
		let msg_id = connector
			.send_message(src_addr, dest_network, dest_addr, gas_limit, gas_cost, payload)
			.await?;
		self.println(
			Some(id),
			format!(
				"sent message {} from {} {} to {} {} with {} gas for {} {}$",
				hex::encode(msg_id),
				src_network,
				self.format_address(Some(src_network), src_addr)?,
				dest_network,
				self.format_address(Some(dest_network), dest_addr)?,
				gas_limit,
				self.format_balance(Some(src_network), gas_cost)?,
				self.config.balance_to_usd(src_network, gas_cost)?,
			),
		)
		.await?;
		Ok(msg_id)
	}

	pub async fn remove_task(&self, task_id: TaskId) -> Result<()> {
		self.runtime.remove_task(task_id).await
	}

	pub async fn complete_batch(&self, batch_id: BatchId, block_hash: BlockHash) -> Result<()> {
		let task_id = self
			.runtime
			.batch_task(batch_id, block_hash)
			.await?
			.context("batch task not found")?;
		let network = self
			.runtime
			.task_network(task_id, block_hash)
			.await?
			.context("task network not found")?;
		let gmp_event = GmpEvent::BatchExecuted {
			batch_id,
			tx_hash: Some([0u8; 32]),
		};
		let events = GmpEvents(BoundedVec::truncate_from(vec![gmp_event]));
		self.runtime.submit_gmp_events(network, events).await
	}

	pub async fn withdraw_funds(
		&self,
		network: NetworkId,
		amount: u128,
		address: Address32,
		block_hash: BlockHash,
	) -> Result<()> {
		let (connector, gateway) = self.gateway(network, block_hash).await?;
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

	async fn deploy_testers(
		&mut self,
		block_hash: BlockHash,
	) -> Result<HashMap<NetworkId, (Address32, u64)>> {
		let mut deploy_tester = FuturesUnordered::new();
		let mut testers = HashMap::new();
		for network in self.iter() {
			if let Ok((address, block)) = self.tester(network) {
				testers.insert(network, (address, block));
			} else {
				let fut = self.deploy_tester(network, block_hash);
				deploy_tester.push(async move {
					let tester = fut.await?;
					Ok::<_, anyhow::Error>((network, tester))
				});
			}
		}
		while let Some(result) = deploy_tester.next().await {
			let (network, tester) = result?;
			testers.insert(network, tester);
		}
		drop(deploy_tester);
		self.config.save_testers(
			testers
				.iter()
				.map(|(n, (a, b))| {
					(*n, (self.format_address(Some(*n), *a).expect("have connector"), *b))
				})
				.collect(),
		)?;
		Ok(testers)
	}

	async fn wait_for_chronicles(&self) -> Result<()> {
		let mut blocks = self.finality_notification_stream();
		let mut id = None;
		loop {
			let Some((hash, _)) = blocks.next().await else {
				continue;
			};
			let chronicles = self.chronicles(hash).await?;
			let online = chronicles.iter().all(|c| c.status == ChronicleStatus::Online);
			tracing::info!("waiting for chronicles to be registered");
			id = Some(self.print_table(id, "chronicles", chronicles).await?);
			if online {
				break;
			}
		}
		Ok(())
	}

	async fn register_all_shards(&self) -> Result<()> {
		self.wait_for_chronicles().await?;
		let mut chronicles_per_network = HashMap::<NetworkId, u16>::new();
		for chronicle in self.config.chronicles() {
			let config = self.chronicle_config(chronicle).await?;
			*chronicles_per_network.entry(config.network).or_default() += 1;
		}

		let mut blocks = self.finality_notification_stream();
		let mut id = None;
		let mut register_shards = FuturesUnordered::new();
		let mut shards_per_network = HashMap::<NetworkId, u16>::new();
		for network in self.iter() {
			let shard_size = self.config.network(network)?.shard_size;
			let chronicles = chronicles_per_network.get(&network).copied().unwrap_or_default();
			let num_shards = chronicles / shard_size;
			shards_per_network.insert(network, num_shards);
			loop {
				let Some((hash, _)) = blocks.next().await else { continue };
				let keys = self.find_online_shard_keys(network, hash).await?;
				if keys.len() == num_shards as usize {
					register_shards.push(self.register_shards(network, keys, hash));
					break;
				}
				let shards = self.shards(hash).await?;
				id = Some(self.print_table(id, "shards", shards).await?);
				tracing::info!(
					"waiting for {}/{} shards to come online for {}",
					keys.len(),
					num_shards,
					network
				);
			}
		}

		while let Some(result) = register_shards.next().await {
			result?;
		}

		for (network, num_shards) in shards_per_network {
			loop {
				let Some((hash, _)) = blocks.next().await else { continue };
				let shards = self.shards(hash).await?;
				let num_registered = shards
					.iter()
					.filter(|shard| {
						shard.network == network
							&& shard.registered == ShardRegistration::Registered
					})
					.count();
				id = Some(self.print_table(id, "shards", shards).await?);
				if num_shards as usize == num_registered {
					break;
				}
				tracing::info!(
					"waiting for {}/{} shards to be registered for {}",
					num_registered,
					num_shards,
					network
				);
			}
		}
		Ok(())
	}

	pub async fn setup_test(&mut self) -> Result<()> {
		let (block_hash, _) = self.latest_block().await?;
		self.deploy(block_hash).await?;
		let (block_hash, _) = self.latest_block().await?;
		self.deploy_testers(block_hash).await?;
		self.register_all_shards().await?;
		Ok(())
	}

	pub fn tester(&self, network: NetworkId) -> Result<(Address32, u64)> {
		let (address, block) = self
			.config
			.tester(network)
			.with_context(|| format!("no tester for {network}"))?;
		Ok((self.parse_address(Some(network), address)?, *block))
	}

	pub async fn wait_for_sync(&self, network: NetworkId) -> Result<()> {
		let mut blocks = self.finality_notification_stream();
		while let Some((hash, _)) = blocks.next().await {
			let status = self.sync_status(network, hash).await?;
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

	pub async fn log(&self, mut query: Query, since: String, limit: Option<u32>) -> Result<()> {
		if let Query::Container { name, .. } = &mut query {
			if let Some(prefix) = self.config.prefix() {
				*name = format!("{prefix}{name}");
			}
		};
		let logs = loki::raw_logs(&query, since, limit).await?;
		match loki::structured_logs(query.filter(), &logs) {
			Ok(logs) => {
				if self.msg.using_slack() {
					self.print_table(None, "logs", logs).await?;
					return Ok(());
				} else {
					let mut text = String::new();
					for log in logs {
						text.push_str(&log.to_string());
						text.push('\n');
					}
					self.println(None, text).await?;
				}
			},
			Err(err) => {
				tracing::error!("failed to parse logs: {err}");
				let mut text = String::new();
				for log in logs {
					text.push_str(&log);
					text.push('\n');
				}
				self.println(None, text).await?;
			},
		}
		Ok(())
	}

	pub async fn debug_transaction(&self, network: NetworkId, hash: Hash) -> Result<String> {
		let connector = self.connector(network).await?;
		connector.debug_transaction(hash).await
	}

	pub async fn assert_reimbursement(&self, block_hash: BlockHash) -> Result<()> {
		// all chronicles should have the configured balance
		for chronicle in self.config.chronicles() {
			let chronicle = self.chronicle_config(chronicle).await?;
			let config = self.config.network(chronicle.network)?;
			let chronicle_funds =
				self.parse_balance(Some(chronicle.network), &config.chronicle_funds)?;
			let balance =
				self.balance(Some(chronicle.network), chronicle.address, block_hash).await?;
			let diff = balance as i128 - chronicle_funds as i128;
			let sign = if diff < 0 { "-" } else { "" };
			tracing::info!(
				"chronicle balance diff {sign}{}",
				self.format_balance(Some(chronicle.network), diff.unsigned_abs())?
			);
			anyhow::ensure!(diff == 0, "reimbursement failed");
		}
		Ok(())
	}

	pub fn total_gateway_funds(&self) -> Result<f64> {
		let mut total_funds = 0.;
		for network in self.iter() {
			let gateway_funds = &self.config.network(network)?.gateway_funds;
			let gateway_funds = self.parse_balance(Some(network), gateway_funds)?;
			total_funds += self.config.balance_to_usd(network, gateway_funds)?;
		}
		Ok(total_funds)
	}

	pub async fn total_gateway_balance(&self, block_hash: BlockHash) -> Result<f64> {
		let mut total_balance = 0.;
		for network in self.iter() {
			let (_connector, gateway) = self.gateway(network, block_hash).await?;
			let balance = self.balance(Some(network), gateway, block_hash).await?;
			total_balance += self.config.balance_to_usd(network, balance)?;
		}
		Ok(total_balance)
	}

	pub async fn exec_smoke(
		&self,
		src: NetworkId,
		dest: NetworkId,
		payload: Vec<u8>,
	) -> Result<GmpMessage> {
		let mut blocks = self.finality_notification_stream();
		let (hash, _) = blocks.next().await.context("expected block")?;
		// prepare
		let src_addr = self.tester(src)?.0;
		let dest_addr = self.tester(dest)?.0;
		let gas_limit = self
			.estimate_message_gas_limit(dest, dest_addr, src, src_addr, payload.clone())
			.await?;
		let gas_cost = self
			.estimate_message_cost(src, dest, payload.len() as u16, gas_limit, hash)
			.await?;

		// send message
		let mut blocks = self.finality_notification_stream();
		let (_, start) = blocks.next().await.context("expected block")?;
		let msg_id = self
			.send_message(src, src_addr, dest, dest_addr, gas_limit, gas_cost, payload.clone())
			.await?;

		// track message
		let mut id = None;
		let (exec, end) = loop {
			let (hash, end) = blocks.next().await.context("expected block")?;
			let trace = self.message_trace(src, msg_id, hash).await?;
			let exec = trace.exec.as_ref().map(|t| t.task);
			tracing::info!("waiting for message {}", hex::encode(msg_id));
			id = Some(self.print_table(id, "message", vec![trace]).await?);
			if let Some(exec) = exec {
				break (exec, end);
			}
		};

		// read message
		let (hash, _) = blocks.next().await.context("expected block")?;
		let blocks = self.read_events_blocks(exec, hash).await?;
		let msgs = self.messages(dest, dest_addr, blocks).await?;
		let msg = msgs
			.into_iter()
			.find(|msg| msg.message_id() == msg_id)
			.context("failed to find message")?;
		self.print_table(None, "message", vec![msg.clone()]).await?;
		self.println(None, format!("received message after {} blocks", end - start))
			.await?;
		Ok(msg)
	}

	pub async fn cost_matrix(&self) -> Result<Vec<RouteCost>> {
		let mut matrix = vec![];
		for src in self.iter() {
			for dest in self.iter() {
				if src == dest {
					continue;
				}
				let dest_connector = self.connector(dest).await?;
				let dest_config = self.config.network(dest)?;
				let msg_cost = self.config.msg_fee(src, dest, 0, 0)?;
				matrix.push(RouteCost {
					src,
					dest,
					dest_gas_price: dest_connector.max_fee_per_gas().await?,
					dest_max_gas_price: dest_config.max_gas_price,
					src_token_usd: self.config.token_price_usd(src)?,
					dest_token_usd: self.config.token_price_usd(dest)?,
					gas_price: self.config.gas_price(src, dest)?,
					msg_gas: dest_config.msg_gas(),
					msg_byte_gas: dest_config.msg_byte_gas(),
					msg_cost,
					msg_cost_usd: self.config.balance_to_usd(src, msg_cost)?,
				});
			}
		}
		Ok(matrix)
	}
}

pub struct RouteCost {
	pub src: NetworkId,
	pub dest: NetworkId,
	pub dest_gas_price: u128,
	pub dest_max_gas_price: u128,
	pub src_token_usd: f64,
	pub dest_token_usd: f64,
	pub gas_price: f64,
	pub msg_gas: u64,
	pub msg_byte_gas: u64,
	pub msg_cost: u128,
	pub msg_cost_usd: f64,
}
