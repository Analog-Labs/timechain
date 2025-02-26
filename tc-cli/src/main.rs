use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tc_cli::{Query, Sender, Tc};
use time_primitives::{Address, BatchId, Hash, NetworkId, ShardId, TaskId};
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
	#[arg(long, default_value = "/etc/envs/local")]
	env: PathBuf,
	#[arg(long, default_value = "config.yaml")]
	config: String,
	#[clap(subcommand)]
	cmd: Command,
}

impl Args {
	async fn tc(&self, sender: Sender) -> Result<Tc> {
		let tc = Tc::new(self.env.clone(), &self.config, sender).await?;
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
	FetchPrices,
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
	UnassignedTasks {
		network: NetworkId,
	},
	AssignedTasks {
		shard: ShardId,
	},
	FailedBatches,
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
	DeployChronicle {
		url: String,
	},
	UnregisterMember {
		member: String,
	},
	RegisterShards {
		network: NetworkId,
	},
	RetryFailedBatch {
		batch_id: BatchId,
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
	EstimateMessageGasLimit {
		dest_network: NetworkId,
		dest_addr: String,
		src_network: NetworkId,
		src_addr: String,
		payload: String,
	},
	EstimateMessageGasCost {
		src_network: NetworkId,
		dest_network: NetworkId,
		gas_limit: u128,
		payload: String,
	},
	SendMessage {
		src_network: NetworkId,
		src_addr: String,
		dest_network: NetworkId,
		dest_addr: String,
		gas_limit: u128,
		gas_cost: u128,
		payload: String,
	},
	SmokeTest {
		src: NetworkId,
		dest: NetworkId,
	},
	SmokeCctp {
		src: NetworkId,
		#[arg(
			long,
			default_value = "000000000000000000000000ab5976445202ac58cdf7da9cb5797a70e6be1cdc"
		)]
		src_address: String,
		dest: NetworkId,
		#[arg(
			long,
			default_value = "000000000000000000000000ab5976445202ac58cdf7da9cb5797a70e6be1cdc"
		)]
		dest_address: String,
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
		query: Query,
		#[arg(long, default_value = "7d")]
		since: String,
	},
	ForceShardOffline {
		shard_id: ShardId,
	},
	DebugTransaction {
		network: NetworkId,
		hash: String,
	},
	DumpState {
		network: NetworkId,
		path: Option<PathBuf>,
	},
	LoadState {
		network: NetworkId,
		path: Option<PathBuf>,
	},
}

#[tokio::main]
async fn main() {
	time_primitives::init_ss58_version();
	rustls::crypto::ring::default_provider()
		.install_default()
		.expect("Failed to install rustls crypto provider");
	if let Err(err) = real_main().await {
		println!("{err:#?}");
		std::io::stdout().flush().unwrap();
		std::process::exit(1);
	} else {
		std::process::exit(0);
	}
}

async fn real_main() -> Result<()> {
	let filter = EnvFilter::from_default_env()
		.add_directive("tc_cli=info".parse().unwrap())
		.add_directive("gmp_evm=info".parse().unwrap());
	tracing_subscriber::fmt().with_env_filter(filter).init();
	let sender = Sender::new();
	let args = Args::parse();
	tracing::info!("main");
	let now = std::time::SystemTime::now();
	let tc = args.tc(sender).await?;
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
			tc.println(None, address).await?;
		},
		Command::Balance { network, address } => {
			let address = if let Some(address) = address {
				tc.parse_address(network, &address)?
			} else {
				tc.address(network)?
			};
			let balance = tc.balance(network, address).await?;
			let balance = tc.format_balance(network, balance)?;
			tc.println(None, balance).await?;
		},
		Command::Transfer { network, address, amount } => {
			let address = tc.parse_address(network, &address)?;
			let amount = tc.parse_balance(network, &amount)?;
			tc.transfer(network, address, amount).await?;
		},
		// read data
		Command::FetchPrices => {
			tc.fetch_token_prices().await?;
		},
		Command::Networks => {
			let networks = tc.networks().await?;
			tc.print_table(None, "networks", networks).await?;
		},
		Command::Chronicles => {
			let chronicles = tc.chronicles().await?;
			tc.print_table(None, "chronicles", chronicles).await?;
		},
		Command::Shards => {
			let shards = tc.shards().await?;
			tc.print_table(None, "shards", shards).await?;
		},
		Command::Members { shard } => {
			let members = tc.members(shard).await?;
			tc.print_table(None, "members", members).await?;
		},
		Command::Routes { network } => {
			let routes = tc.routes(network).await?;
			tc.print_table(None, "routes", routes).await?;
		},
		Command::Events { network, start, end } => {
			let events = tc.events(network, start..end).await?;
			tc.print_table(None, "events", events).await?;
		},
		Command::Messages { network, tester, start, end } => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let msgs = tc.messages(network, tester, start..end).await?;
			tc.print_table(None, "messages", msgs).await?;
		},
		Command::Task { task } => {
			let task = tc.task(task).await?;
			tc.print_table(None, "task", vec![task]).await?;
		},
		Command::UnassignedTasks { network } => {
			let tasks = tc.unassigned_tasks(network).await?;
			tc.print_table(None, "unassigned-tasks", tasks).await?;
		},
		Command::AssignedTasks { shard } => {
			let tasks = tc.assigned_tasks(shard).await?;
			tc.print_table(None, "assigned-tasks", tasks).await?;
		},
		Command::FailedBatches => {
			let batches = tc.get_failed_batches().await?;
			tc.print_table(None, "failed-batches", batches).await?;
		},
		Command::TransactionBaseFee { network } => {
			let base_fee = tc.transaction_base_fee(network).await?;
			tc.println(
				None,
				format!("Transaction base fee for network: {} is : {}", network, base_fee),
			)
			.await?;
		},
		Command::Batch { batch } => {
			let mut batch = tc.batch(batch).await?;
			let ops = std::mem::take(&mut batch.msg.ops);
			tc.print_table(None, "batch", vec![batch]).await?;
			tc.print_table(None, "ops", ops).await?;
		},

		Command::BlockGasLimit { network } => {
			let base_fee = tc.block_gas_limit(network).await?;
			tc.println(None, format!("Gas limit for block: {} is : {}", network, base_fee))
				.await?;
		},
		Command::Message { message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let message = tc.message(message).await?;
			tc.print_table(None, "messages", vec![message]).await?;
		},
		Command::MessageTrace { network, message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let trace = tc.message_trace(network, message).await?;
			tc.print_table(None, "message", vec![trace]).await?;
		},
		// management
		Command::RuntimeUpgrade { path } => {
			tc.runtime_upgrade(&path).await?;
		},
		Command::Deploy => {
			tc.deploy().await?;
		},
		Command::DeployChronicle { url } => {
			tc.deploy_chronicle(&url).await?;
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
			tc.println(None, format!("{address} {block}")).await?;
		},
		Command::RemoveTask { task_id } => tc.remove_task(task_id).await?,
		Command::CompleteBatch { network_id, batch_id } => {
			tc.complete_batch(network_id, batch_id).await?
		},
		Command::EstimateMessageGasLimit {
			dest_network,
			dest_addr,
			src_network,
			src_addr,
			payload,
		} => {
			let src_addr = tc.parse_address(Some(src_network), &src_addr)?;
			let dest_addr = tc.parse_address(Some(dest_network), &dest_addr)?;
			let payload = hex::decode(payload)?;
			let gas_limit = tc
				.estimate_message_gas_limit(dest_network, dest_addr, src_network, src_addr, payload)
				.await?;
			tc.println(None, gas_limit.to_string()).await?;
		},
		Command::EstimateMessageGasCost {
			src_network,
			dest_network,
			gas_limit,
			payload,
		} => {
			let payload = hex::decode(payload)?;
			let gas_cost =
				tc.estimate_message_cost(src_network, dest_network, gas_limit, payload).await?;
			tc.println(None, gas_cost.to_string()).await?;
		},
		Command::SendMessage {
			src_network,
			src_addr,
			dest_network,
			dest_addr,
			gas_limit,
			gas_cost,
			payload,
		} => {
			let src_addr = tc.parse_address(Some(src_network), &src_addr)?;
			let dest_addr = tc.parse_address(Some(dest_network), &dest_addr)?;
			let payload = hex::decode(payload)?;
			let msg_id = tc
				.send_message(
					src_network,
					src_addr,
					dest_network,
					dest_addr,
					gas_limit,
					gas_cost,
					payload,
				)
				.await?;
			tc.println(None, hex::encode(msg_id)).await?;
		},
		Command::SmokeTest { src, dest } => {
			let (src_addr, dest_addr) = tc.setup_test(src, dest).await?;
			exec_smoke(tc, src, src_addr, dest, dest_addr, SmokeType::Gmp).await?;
		},
		Command::SmokeCctp {
			src,
			src_address,
			dest,
			dest_address,
		} => {
			tc.setup_test(src, dest).await?;
			let src_addr = hex::decode(src_address)
				.unwrap()
				.try_into()
				.expect("Unable to convert src_address to bytes32");
			let dest_addr = hex::decode(dest_address)
				.unwrap()
				.try_into()
				.expect("Unable to convert dest_address to bytes32");
			exec_smoke(tc, src, src_addr, dest, dest_addr, SmokeType::Cctp).await?;
		},
		Command::Benchmark { src, dest, num_messages } => {
			let (src_addr, dest_addr) = tc.setup_test(src, dest).await?;
			tc.wait_for_sync(src).await?;
			tc.wait_for_sync(dest).await?;
			let mut blocks = tc.finality_notification_stream();
			let (_, start) = blocks.next().await.context("expected block")?;
			let mut msgs = HashSet::new();
			let payload = vec![];
			let gas_limit = tc
				.estimate_message_gas_limit(dest, dest_addr, src, src_addr, payload.clone())
				.await?;
			let gas_cost = tc.estimate_message_cost(src, dest, gas_limit, payload.clone()).await?;
			for _ in 0..num_messages {
				let msg_id = tc
					.send_message(
						src,
						src_addr,
						dest,
						dest_addr,
						gas_limit,
						gas_cost,
						payload.clone(),
					)
					.await?;
				msgs.insert(msg_id);
			}
			let mut id = None;
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
				id = Some(
					tc.println(
						id,
						format!(
							r#"{num_received} out of {num_messages} received in {blocks} blocks
							throughput {throughput:.3} msgs/block
							msg cost {}"#,
							tc.format_balance(Some(src), gas_cost)?,
						),
					)
					.await?,
				);
				if msgs.is_empty() {
					break;
				}
			}
		},
		Command::Log { query, since } => {
			tc.log(query, since).await?;
		},
		Command::ForceShardOffline { shard_id } => {
			tc.force_shard_offline(shard_id).await?;
		},
		Command::WithdrawFunds { network, amount, address } => {
			let address = tc.parse_address(Some(network), &address)?;
			tc.withdraw_funds(network, amount, address).await?;
		},
		Command::DebugTransaction { network, hash } => {
			let hash = hash.strip_prefix("0x").unwrap_or(&hash);
			let hash: Hash =
				hex::decode(hash)?.try_into().map_err(|_| anyhow::anyhow!("invalid hash"))?;
			let output = tc.debug_transaction(network, hash).await?;
			tc.println(None, output).await?;
		},
		Command::RetryFailedBatch { batch_id } => {
			tc.restart_failed_batch(batch_id).await?;
		},
		Command::DumpState { network, path } => {
			let path = path.unwrap_or("anvil_state.txt".into());
			let state = tc.dump_state(network).await?;
			std::fs::write(&path, state)?;
			tracing::info!("Anvil state stored to: {:?}", &path);
		},
		Command::LoadState { network, path } => {
			let path = path.unwrap_or("anvil_state.txt".into());
			let state = std::fs::read_to_string(&path)?;
			tc.load_state(network, state).await?;
			tracing::info!("Anvil state loaded from: {:?}", &path);
		},
	}
	tracing::info!("executed query in {}s", now.elapsed().unwrap().as_secs());
	Ok(())
}

async fn exec_smoke(
	tc: Tc,
	src: NetworkId,
	src_addr: Address,
	dest: NetworkId,
	dest_addr: Address,
	smoke_type: SmokeType,
) -> Result<()> {
	const CCTP_MSG_LEN: usize = 896;
	let mut blocks = tc.finality_notification_stream();
	let (_, start) = blocks.next().await.context("expected block")?;
	let payload = vec![0u8; CCTP_MSG_LEN];
	let gas_limit = tc
		.estimate_message_gas_limit(dest, dest_addr, src, src_addr, payload.clone())
		.await?;
	let gas_cost = tc.estimate_message_cost(src, dest, gas_limit, payload.clone()).await?;
	let msg_id = match smoke_type {
		SmokeType::Gmp => {
			tc.send_message(src, src_addr, dest, dest_addr, gas_limit, gas_cost, payload.clone())
				.await?
		},
		SmokeType::Cctp => {
			tc.send_cctp_message(src, src_addr, dest, dest_addr, gas_limit, gas_cost)
				.await?
		},
	};
	let mut id = None;
	let (exec, end) = loop {
		let (_, end) = blocks.next().await.context("expected block")?;
		let trace = tc.message_trace(src, msg_id).await?;
		let exec = trace.exec.as_ref().map(|t| t.task);
		tracing::info!("waiting for message {}", hex::encode(msg_id));
		id = Some(tc.print_table(id, "message", vec![trace]).await?);
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
	tc.print_table(None, "message", vec![msg]).await?;
	tc.println(None, format!("received message after {} blocks", end - start))
		.await?;
	Ok(())
}

enum SmokeType {
	Gmp,
	Cctp,
}
