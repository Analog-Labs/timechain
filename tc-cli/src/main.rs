use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::{Table, Tabled};
use tc_cli::{
	Batch, Chronicle, ChronicleStatus, Member, Message, MessageTrace, Network, Shard, Task, Tc,
};
use time_primitives::{
	traits::IdentifyAccount, Address, BatchId, GatewayOp, GmpEvent, GmpMessage, NetworkId, Route,
	ShardId, TaskId,
};
use tracing_subscriber::filter::EnvFilter;

#[derive(Clone, Debug)]
pub struct RelGasPrice {
	pub num: u128,
	pub den: u128,
}

impl FromStr for RelGasPrice {
	type Err = anyhow::Error;

	fn from_str(rel_gas_price: &str) -> Result<Self> {
		let (num, den) = rel_gas_price.split_once('/').context(
			"invalid relative gas price, expected a ratio of unsigned integers separated by '/'",
		)?;
		Ok(Self {
			num: num.parse()?,
			den: den.parse()?,
		})
	}
}

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_value = "/etc/config.yaml")]
	config: PathBuf,
	#[clap(subcommand)]
	cmd: Command,
}

impl Args {
	async fn tc(&self) -> Result<Tc> {
		let tc = Tc::new(&self.config).await?;
		Ok(tc)
	}
}

#[derive(Parser, Debug)]
enum Command {
	// balances
	Address {
		#[arg(long)]
		network: Option<NetworkId>,
	},
	Faucet {
		network: NetworkId,
	},
	Balance {
		#[arg(long)]
		network: Option<NetworkId>,
		#[arg(long)]
		address: Option<String>,
	},
	Transfer {
		#[arg(long)]
		network: Option<NetworkId>,
		address: String,
		amount: String,
	},
	// read data
	FetchTokenPriceData,
	Networks,
	Chronicles,
	Shards,
	Members {
		shard: ShardId,
	},
	Routes {
		network: NetworkId,
	},
	Events {
		network: NetworkId,
		start: u64,
		end: u64,
	},
	Messages {
		network: NetworkId,
		tester: String,
		start: u64,
		end: u64,
	},
	Task {
		task: TaskId,
	},
	TransactionBaseFee {
		network: NetworkId,
	},
	Batch {
		batch: BatchId,
	},
	BlockGasLimit {
		network: NetworkId,
	},
	Message {
		message: String,
	},
	MessageTrace {
		network: NetworkId,
		message: String,
	},
	// management
	RuntimeUpgrade {
		path: PathBuf,
	},
	Deploy,
	UnregisterMember {
		member: String,
	},
	RegisterShards {
		network: NetworkId,
	},
	SetGatewayAdmin {
		network: NetworkId,
		admin: String,
	},
	RedeployGateway {
		network: NetworkId,
	},
	DeployTester {
		network: NetworkId,
	},
	RemoveTask {
		task_id: TaskId,
	},
	CompleteBatch {
		network_id: NetworkId,
		batch_id: BatchId,
	},
	SendMessage {
		network: NetworkId,
		tester: String,
		dest: NetworkId,
		dest_addr: String,
		nonce: u64,
	},
	SmokeTest {
		src: NetworkId,
		dest: NetworkId,
	},
	WithdrawFunds {
		network: NetworkId,
		amount: u128,
		address: String,
	},
	Benchmark {
		src: NetworkId,
		dest: NetworkId,
		num_messages: u16,
	},
	Log {
		#[clap(subcommand)]
		query: tc_cli::loki::Query,
	},
	ForceShardOffline {
		shard_id: ShardId,
	},
}

trait IntoRow {
	type Row: Tabled;

	fn into_row(self, tc: &Tc) -> Result<Self::Row>;
}

fn print_table<R: IntoRow>(tc: &Tc, table: Vec<R>) -> Result<()> {
	let mut rows = Vec::with_capacity(table.len());
	for row in table {
		rows.push(row.into_row(tc)?);
	}
	println!("{}", Table::new(rows));
	Ok(())
}

#[derive(Tabled)]
struct NetworkEntry {
	network: NetworkId,
	chain_name: String,
	chain_network: String,
	gateway: String,
	gateway_balance: String,
	admin: String,
	admin_balance: String,
	read_events: TaskId,
	sync_status: String,
}

impl IntoRow for Network {
	type Row = NetworkEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let mut gateway = String::new();
		let mut gateway_balance = String::new();
		let mut admin = String::new();
		let mut admin_balance = String::new();
		let mut read_events = 0;
		let sync_status = if let Some(info) = self.info.as_ref() {
			gateway = tc.format_address(Some(self.network), info.gateway)?;
			gateway_balance = tc.format_balance(Some(self.network), info.gateway_balance)?;
			admin = tc.format_address(Some(self.network), info.admin)?;
			admin_balance = tc.format_balance(Some(self.network), info.admin_balance)?;
			read_events = info.sync_status.task;
			format!("{} / {}", info.sync_status.sync, info.sync_status.block)
		} else {
			"no connector configured".to_string()
		};
		Ok(NetworkEntry {
			network: self.network,
			chain_name: self.chain_name,
			chain_network: self.chain_network,
			gateway,
			gateway_balance,
			admin,
			admin_balance,
			read_events,
			sync_status,
		})
	}
}

#[derive(Tabled)]
struct ChronicleEntry {
	network: NetworkId,
	account: String,
	peer_id: String,
	status: String,
	balance: String,
	target_address: String,
	target_balance: String,
}

impl IntoRow for Chronicle {
	type Row = ChronicleEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(ChronicleEntry {
			network: self.network,
			account: tc.format_address(None, self.account.into())?,
			peer_id: self.peer_id,
			status: self.status.to_string(),
			balance: tc.format_balance(None, self.balance)?,
			target_address: tc.format_address(Some(self.network), self.target_address)?,
			target_balance: tc.format_balance(Some(self.network), self.target_balance)?,
		})
	}
}

#[derive(Tabled)]
struct ShardEntry {
	shard: ShardId,
	network: NetworkId,
	status: String,
	key: String,
	registered: String,
	size: u16,
	threshold: u16,
}

impl IntoRow for Shard {
	type Row = ShardEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(ShardEntry {
			shard: self.shard,
			network: self.network,
			status: self.status.to_string(),
			key: self.key.map(hex::encode).unwrap_or_default(),
			registered: self.registered.to_string(),
			size: self.size,
			threshold: self.threshold,
		})
	}
}

#[derive(Tabled)]
struct MemberEntry {
	account: String,
	status: String,
	staker: String,
	stake: String,
}

impl IntoRow for Member {
	type Row = MemberEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(MemberEntry {
			account: self.account.to_string(),
			status: self.status.to_string(),
			staker: self
				.staker
				.map(|staker| tc.format_address(None, staker.into()))
				.transpose()?
				.unwrap_or_default(),
			stake: tc.format_balance(None, self.stake)?,
		})
	}
}

#[derive(Tabled)]
struct RouteEntry {
	network: NetworkId,
	gateway: String,
	relative_gas_price: String,
	gas_limit: u64,
	base_fee: u128,
}

impl IntoRow for Route {
	type Row = RouteEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let (num, den) = self.relative_gas_price;
		Ok(RouteEntry {
			network: self.network_id,
			gateway: tc.format_address(Some(self.network_id), self.gateway)?,
			relative_gas_price: format!("{}", num as f64 / den as f64),
			gas_limit: self.gas_limit,
			base_fee: self.base_fee,
		})
	}
}

#[derive(Tabled)]
struct EventEntry {
	event: String,
}

impl IntoRow for GmpEvent {
	type Row = EventEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(EventEntry { event: self.to_string() })
	}
}

#[derive(Tabled)]
struct MessageEntry {
	id: String,
	source_network: NetworkId,
	source_address: String,
	dest_network: NetworkId,
	dest_address: String,
}

impl IntoRow for GmpMessage {
	type Row = MessageEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(MessageEntry {
			id: self.to_string(),
			source_network: self.src_network,
			source_address: tc.format_address(Some(self.src_network), self.src)?,
			dest_network: self.dest_network,
			dest_address: tc.format_address(Some(self.dest_network), self.dest)?,
		})
	}
}

#[derive(Tabled)]
struct TaskEntry {
	task: TaskId,
	network: NetworkId,
	descriptor: String,
	output: String,
	shard: String,
	submitter: String,
}

impl IntoRow for Task {
	type Row = TaskEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(TaskEntry {
			task: self.task,
			network: self.network,
			descriptor: self.descriptor.to_string(),
			output: match self.output {
				Some(Ok(())) => "complete".to_string(),
				Some(Err(err)) => err,
				None => "in progress".to_string(),
			},
			shard: match self.shard {
				Some(shard) => shard.to_string(),
				None => "unassigned".to_string(),
			},
			submitter: match self.submitter {
				Some(submitter) => tc.format_address(None, submitter.into_account().into())?,
				None => "".to_string(),
			},
		})
	}
}

#[derive(Tabled)]
struct BatchEntry {
	batch: BatchId,
	task: TaskId,
}

impl IntoRow for Batch {
	type Row = BatchEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchEntry {
			batch: self.batch,
			task: self.task,
		})
	}
}

#[derive(Tabled)]
struct BatchOpEntry {
	op: String,
}

impl IntoRow for GatewayOp {
	type Row = BatchOpEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchOpEntry { op: self.to_string() })
	}
}

#[derive(Tabled)]
struct MessageInfoEntry {
	message: String,
	recv: String,
	batch: String,
	exec: String,
}

impl IntoRow for Message {
	type Row = MessageInfoEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(MessageInfoEntry {
			message: hex::encode(self.message),
			recv: self.recv.map(|t| t.to_string()).unwrap_or_default(),
			batch: self.batch.map(|b| b.to_string()).unwrap_or_default(),
			exec: self.exec.map(|t| t.to_string()).unwrap_or_default(),
		})
	}
}

#[derive(Tabled)]
struct MessageTraceEntry {
	message: String,
	src_sync: String,
	dest_sync: String,
	recv: String,
	submit: String,
	exec: String,
}

impl IntoRow for MessageTrace {
	type Row = MessageTraceEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		fn task_to_string(task: Task) -> String {
			let status = if let Some(output) = task.output {
				match output {
					Ok(()) => "complete".to_string(),
					Err(err) => format!("failed '{err}'"),
				}
			} else if let Some(shard) = task.shard {
				format!("assigned to {}", shard)
			} else {
				"unassigned".to_string()
			};
			format!("{} {}", task.task, status)
		}
		Ok(MessageTraceEntry {
			message: hex::encode(self.message),
			src_sync: format!("{} / {}", self.src.sync, self.src.block),
			dest_sync: if let Some(sync) = self.dest {
				format!("{} / {}", sync.sync, sync.block)
			} else {
				"- / -".into()
			},
			recv: self.recv.map(task_to_string).unwrap_or_default(),
			submit: self.submit.map(task_to_string).unwrap_or_default(),
			exec: self.exec.map(task_to_string).unwrap_or_default(),
		})
	}
}

#[tokio::main]
async fn main() {
	if let Err(err) = real_main().await {
		println!("{err:#?}");
		std::process::exit(1);
	} else {
		std::process::exit(0);
	}
}

async fn real_main() -> Result<()> {
	let filter = EnvFilter::from_default_env().add_directive("tc_cli=info".parse()?);
	tracing_subscriber::fmt().with_env_filter(filter).init();
	let args = Args::parse();
	tracing::info!("main");
	let now = std::time::SystemTime::now();
	let tc = args.tc().await?;
	tracing::info!("tc ready in {}s", now.elapsed().unwrap().as_secs());
	let now = std::time::SystemTime::now();
	match args.cmd {
		// balances
		Command::Faucet { network } => {
			tc.faucet(network).await?;
		},
		Command::Address { network } => {
			let address = tc.address(network)?;
			let address = tc.format_address(network, address)?;
			println!("{address}");
		},
		Command::Balance { network, address } => {
			let address = if let Some(address) = address {
				tc.parse_address(network, &address)?
			} else {
				tc.address(network)?
			};
			let balance = tc.balance(network, address).await?;
			let balance = tc.format_balance(network, balance)?;
			println!("{balance}");
		},
		Command::Transfer { network, address, amount } => {
			let address = tc.parse_address(network, &address)?;
			let amount = tc.parse_balance(network, &amount)?;
			tc.transfer(network, address, amount).await?;
		},
		// read data
		Command::FetchTokenPriceData => {
			tc.fetch_token_prices().await?;
		},
		Command::Networks => {
			let networks = tc.networks().await?;
			print_table(&tc, networks)?;
		},
		Command::Chronicles => {
			let chronicles = tc.chronicles().await?;
			print_table(&tc, chronicles)?;
		},
		Command::Shards => {
			let shards = tc.shards().await?;
			print_table(&tc, shards)?;
		},
		Command::Members { shard } => {
			let members = tc.members(shard).await?;
			print_table(&tc, members)?;
		},
		Command::Routes { network } => {
			let routes = tc.routes(network).await?;
			print_table(&tc, routes)?;
		},
		Command::Events { network, start, end } => {
			let events = tc.events(network, start..end).await?;
			print_table(&tc, events)?;
		},
		Command::Messages { network, tester, start, end } => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let msgs = tc.messages(network, tester, start..end).await?;
			print_table(&tc, msgs)?;
		},
		Command::Task { task } => {
			let task = tc.task(task).await?;
			print_table(&tc, vec![task])?;
		},
		Command::TransactionBaseFee { network } => {
			let base_fee = tc.transaction_base_fee(network).await?;
			println!("Transaction base fee for network: {} is : {}", network, base_fee);
		},
		Command::Batch { batch } => {
			let mut batch = tc.batch(batch).await?;
			let ops = std::mem::take(&mut batch.msg.ops);
			print_table(&tc, vec![batch])?;
			print_table(&tc, ops)?;
		},

		Command::BlockGasLimit { network } => {
			let base_fee = tc.block_gas_limit(network).await?;
			println!("Gas limit for block: {} is : {}", network, base_fee);
		},
		Command::Message { message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let message = tc.message(message).await?;
			print_table(&tc, vec![message])?;
		},
		Command::MessageTrace { network, message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let trace = tc.message_trace(network, message).await?;
			print_table(&tc, vec![trace])?;
		},
		// management
		Command::RuntimeUpgrade { path } => {
			tc.runtime_upgrade(&path).await?;
		},
		Command::Deploy => {
			tc.deploy(None).await?;
		},
		Command::UnregisterMember { member } => {
			let member = tc.parse_address(None, &member)?;
			tc.unregister_member(member.into()).await?;
		},
		Command::RegisterShards { network } => {
			tc.register_shards(network).await?;
		},
		Command::SetGatewayAdmin { network, admin } => {
			let admin = tc.parse_address(Some(network), &admin)?;
			tc.set_gateway_admin(network, admin).await?;
		},
		Command::RedeployGateway { network } => {
			tc.redeploy_gateway(network).await?;
		},
		Command::DeployTester { network } => {
			let (address, block) = tc.deploy_tester(network).await?;
			let address = tc.format_address(Some(network), address)?;
			println!("{address} {block}");
		},
		Command::RemoveTask { task_id } => tc.remove_task(task_id).await?,
		Command::CompleteBatch { network_id, batch_id } => {
			tc.complete_batch(network_id, batch_id).await?
		},
		Command::SendMessage {
			network,
			tester,
			dest,
			dest_addr,
			nonce,
		} => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let dest_addr = tc.parse_address(Some(dest), &dest_addr)?;
			let msg_id = tc.send_message(network, tester, dest, dest_addr, nonce).await?;
			println!("{}", hex::encode(msg_id));
		},
		Command::SmokeTest { src, dest } => {
			let (src_addr, dest_addr) = setup(&tc, src, dest).await?;
			let mut blocks = tc.finality_notification_stream();
			let (_, start) = blocks.next().await.context("expected block")?;
			let msg_id = tc.send_message(src, src_addr, dest, dest_addr, 0).await?;
			tracing::info!(
				"sent message to {} {}",
				dest,
				tc.format_address(Some(dest), dest_addr)?
			);
			let (exec, end) = loop {
				let (_, end) = blocks.next().await.context("expected block")?;
				let trace = tc.message_trace(src, msg_id).await?;
				let exec = trace.exec.as_ref().map(|t| t.task);
				tracing::info!("waiting for message {}", hex::encode(msg_id));
				print_table(&tc, vec![trace])?;
				if let Some(exec) = exec {
					break (exec, end);
				}
			};
			let blocks = tc.read_events_blocks(exec).await?;
			let msgs = tc.messages(dest, dest_addr, blocks).await?;
			let msg = msgs
				.into_iter()
				.find(|msg| msg.message_id() == msg_id)
				.context("failed to find message")?;
			print_table(&tc, vec![msg])?;
			println!("received message after {} blocks", end - start);
		},
		Command::Benchmark { src, dest, num_messages } => {
			let (src_addr, dest_addr) = setup(&tc, src, dest).await?;
			wait_for_sync(&tc, src).await?;
			wait_for_sync(&tc, dest).await?;
			let mut blocks = tc.finality_notification_stream();
			let (_, start) = blocks.next().await.context("expected block")?;
			let mut msgs = HashSet::new();
			for nonce in 0..num_messages {
				let msg = tc.send_message(src, src_addr, dest, dest_addr, nonce as _).await?;
				msgs.insert(msg);
			}
			while let Some((_, block)) = blocks.next().await {
				let mut received = HashSet::new();
				for msg in &msgs {
					let msg = tc.message(*msg).await?;
					if msg.exec.is_some() {
						received.insert(msg.message);
					}
				}
				for msg in received {
					msgs.remove(&msg);
				}
				let num_received = num_messages - msgs.len() as u16;
				let blocks = block - start;
				let throughput = num_received as f64 / blocks as f64;
				println!("{num_received} out of {num_messages} received in {blocks} blocks");
				println!("throughput {throughput:.3} msgs/block");
				if msgs.is_empty() {
					break;
				}
			}
		},
		Command::Log { query } => {
			for log in tc_cli::loki::logs(query).await? {
				println!("{log}");
			}
		},
		Command::ForceShardOffline { shard_id } => {
			tc.force_shard_offline(shard_id).await?;
		},
		Command::WithdrawFunds { network, amount, address } => {
			tc.withdraw_funds(network, amount, address).await?;
		},
	}
	tracing::info!("executed query in {}s", now.elapsed().unwrap().as_secs());
	Ok(())
}

async fn setup(tc: &Tc, src: NetworkId, dest: NetworkId) -> Result<(Address, Address)> {
	// networks
	tc.deploy(Some(vec![src, dest])).await?;
	let (src_addr, src_block) = tc.deploy_tester(src).await?;
	let (dest_addr, dest_block) = tc.deploy_tester(dest).await?;
	tracing::info!("deployed at src block {}, dest block {}", src_block, dest_block);
	let networks = tc.networks().await?;
	print_table(tc, networks.clone())?;
	for network in networks {
		let routes = tc.routes(network.network).await?;
		print_table(tc, routes)?;
	}
	// chronicles
	let mut blocks = tc.finality_notification_stream();
	while blocks.next().await.is_some() {
		let chronicles = tc.chronicles().await?;
		let not_registered = chronicles.iter().any(|c| c.status != ChronicleStatus::Online);
		tracing::info!("waiting for chronicles to be registered");
		print_table(tc, chronicles)?;
		if !not_registered {
			break;
		}
	}
	// shards
	while blocks.next().await.is_some() {
		let src_keys = tc.find_online_shard_keys(src).await?;
		let dest_keys = tc.find_online_shard_keys(dest).await?;
		if !src_keys.is_empty() && !dest_keys.is_empty() {
			break;
		}
		tracing::info!("waiting for shards to come online");
		let shards = tc.shards().await?;
		print_table(tc, shards)?;
	}
	// registered shards
	tc.register_shards(src).await?;
	tc.register_shards(dest).await?;
	while blocks.next().await.is_some() {
		let shards = tc.shards().await?;
		let is_registered = shards.iter().any(|shard| shard.registered && shard.network == dest);
		tracing::info!("waiting for shard to be registered");
		print_table(tc, shards)?;
		if is_registered {
			break;
		}
	}
	Ok((src_addr, dest_addr))
}

async fn wait_for_sync(tc: &Tc, network: NetworkId) -> Result<()> {
	let mut blocks = tc.finality_notification_stream();
	while blocks.next().await.is_some() {
		let status = tc.sync_status(network).await?;
		tracing::info!("waiting for network {network} to sync {} / {}", status.sync, status.block);
		if status.next_sync > status.block {
			break;
		}
	}
	Ok(())
}
