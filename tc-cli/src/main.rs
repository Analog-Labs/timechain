use anyhow::{Context, Result};
use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tc_cli::{Benchmark, Query, Sender, Tc};
use time_primitives::{BatchId, BlockNumber, Hash, NetworkId, ShardId, TaskId};
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
	#[arg(long, default_value = "cached_tx.redb")]
	db: PathBuf,
	#[clap(subcommand)]
	cmd: Command,
}

#[derive(Parser, Debug)]
#[allow(clippy::large_enum_variant)]
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
	PendingBatches,
	MaxFeePerGas {
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
	RegisterShards,
	RegisterRoutes,
	RestartBatch {
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
	SetTcRoute {
		src_network: NetworkId,
		src_gateway: String,
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
		#[arg(long, default_value = "10")]
		num_messages_per_block: u16,
		#[arg(long, default_value = "10")]
		num_blocks: BlockNumber,
	},
	Log {
		#[clap(subcommand)]
		query: Query,
		#[arg(long, default_value = "7d")]
		since: String,
		#[arg(long, default_value = "100")]
		limit: u32,
	},
	ForceShardOffline {
		shard_id: ShardId,
	},
	DebugTransaction {
		network: NetworkId,
		hash: String,
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
	let filter = EnvFilter::from_default_env().add_directive("info".parse()?);
	tracing_subscriber::fmt().with_env_filter(filter).init();
	let args = Args::parse();
	tracing::debug!("main");
	let now = std::time::SystemTime::now();
	let mut tc = Tc::from_env(args.env.clone(), &args.config, Sender::new(), args.db).await?;
	tracing::debug!("tc ready in {}s", now.elapsed().unwrap().as_secs());
	let now = std::time::SystemTime::now();
	let block = tc.latest_block().await?.0;
	match args.cmd {
		// balances
		Command::Faucet { network } => {
			tc.faucet(network, block).await?;
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
			let balance = tc.balance(network, address, block).await?;
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
			let networks = tc.networks(block).await?;
			tc.print_table(None, "networks", networks).await?;
		},
		Command::Chronicles => {
			let chronicles = tc.chronicles(block).await?;
			tc.print_table(None, "chronicles", chronicles).await?;
		},
		Command::Shards => {
			let shards = tc.shards(block).await?;
			tc.print_table(None, "shards", shards).await?;
		},
		Command::Members { shard } => {
			let members = tc.members(shard, block).await?;
			tc.print_table(None, "members", members).await?;
		},
		Command::Routes { network } => {
			let routes = tc.routes(network, block).await?;
			tc.print_table(None, "routes", routes).await?;
		},
		Command::Events { network, start, end } => {
			let events = tc.events(network, start..end, block).await?;
			tc.print_table(None, "events", events).await?;
		},
		Command::Messages { network, tester, start, end } => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let msgs = tc.messages(network, tester, start..end).await?;
			tc.print_table(None, "messages", msgs).await?;
		},
		Command::Task { task } => {
			let task = tc.task(task, block).await?;
			tc.print_table(None, "task", vec![task]).await?;
		},
		Command::UnassignedTasks { network } => {
			let tasks = tc.unassigned_tasks(network, block).await?;
			tc.print_table(None, "unassigned-tasks", tasks).await?;
		},
		Command::AssignedTasks { shard } => {
			let tasks = tc.assigned_tasks(shard, block).await?;
			tc.print_table(None, "assigned-tasks", tasks).await?;
		},
		Command::FailedBatches => {
			let batches = tc.get_failed_batches(block).await?;
			tc.print_table(None, "failed-batches", batches).await?;
		},
		Command::PendingBatches => {
			let batches = tc.get_pending_batches(block).await?;
			tc.print_table(None, "pending-batches", batches).await?;
		},
		Command::MaxFeePerGas { network } => {
			let fee = tc.max_fee_per_gas(network).await?;
			tc.println(
				None,
				format!("EIP1559 max_fee_per_gas for network: {} is : {}", network, fee),
			)
			.await?;
		},
		Command::Batch { batch } => {
			let mut batch = tc.batch(batch, block).await?;
			let ops = std::mem::take(&mut batch.msg.ops);
			tc.print_table(None, "batch", vec![batch]).await?;
			tc.print_table(None, "ops", ops).await?;
		},

		Command::BlockGasLimit { network } => {
			let limit = tc.block_gas_limit(network).await?;
			tc.println(None, format!("Gas limit for block: {} is : {}", network, limit))
				.await?;
		},
		Command::Message { message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let message = tc.message(message, block).await?;
			tc.print_table(None, "messages", vec![message]).await?;
		},
		Command::MessageTrace { network, message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let trace = tc.message_trace(network, message, block).await?;
			tc.print_table(None, "message", vec![trace]).await?;
		},
		// management
		Command::RuntimeUpgrade { path } => {
			tc.runtime_upgrade(&path).await?;
		},
		Command::Deploy => {
			tc.deploy(block).await?;
		},
		Command::DeployChronicle { url } => {
			tc.deploy_chronicle(&url, block).await?;
		},
		Command::UnregisterMember { member } => {
			let member = tc.parse_address(None, &member)?;
			tc.unregister_member(member.into(), block).await?;
		},
		Command::RegisterShards => {
			tc.register_online_shards(block).await?;
		},
		Command::RegisterRoutes => tc.register_all_routes(block).await?,
		Command::SetGatewayAdmin { network, admin } => {
			let admin = tc.parse_address(Some(network), &admin)?;
			tc.set_gateway_admin(network, admin, block).await?;
		},
		Command::RedeployGateway { network } => {
			tc.redeploy_gateway(network, block).await?;
		},
		Command::DeployTester { network } => {
			let (address, block) = tc.deploy_tester(network, block).await?;
			let address = tc.format_address(Some(network), address)?;
			tc.println(None, format!("{address} {block}")).await?;
		},
		Command::RemoveTask { task_id } => tc.remove_task(task_id).await?,
		Command::CompleteBatch { batch_id } => tc.complete_batch(batch_id, block).await?,
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
			let gas_cost = tc
				.estimate_message_cost(src_network, dest_network, gas_limit, payload, block)
				.await?;
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
		Command::SetTcRoute { src_network, src_gateway } => {
			let src_gateway = tc.parse_address(Some(src_network), &src_gateway)?;
			tc.set_tc_route(src_network, src_gateway).await?;
		},
		Command::SmokeTest { src, dest } => {
			tc.setup_test().await?;
			let _ = tc.exec_smoke(src, dest, vec![42]).await?;
		},
		Command::Benchmark {
			num_messages_per_block,
			num_blocks,
		} => {
			tc.setup_test().await?;
			let (block_hash, _) = tc.latest_block().await?;
			let mut benchmark = Benchmark::new(tc, vec![42], num_messages_per_block, num_blocks);
			benchmark.add_routes(block_hash).await?;
			benchmark.wait_for_sync().await?;
			benchmark.exec().await?;
		},
		Command::Log { query, since, limit } => {
			tc.log(query, since, Some(limit)).await?;
		},
		Command::ForceShardOffline { shard_id } => {
			tc.force_shard_offline(shard_id, block).await?;
		},
		Command::WithdrawFunds { network, amount, address } => {
			let address = tc.parse_address(Some(network), &address)?;
			tc.withdraw_funds(network, amount, address, block).await?;
		},
		Command::DebugTransaction { network, hash } => {
			let hash = hash.strip_prefix("0x").unwrap_or(&hash);
			let hash: Hash =
				hex::decode(hash)?.try_into().map_err(|_| anyhow::anyhow!("invalid hash"))?;
			let output = tc.debug_transaction(network, hash).await?;
			tc.println(None, output).await?;
		},
		Command::RestartBatch { batch_id } => {
			tc.restart_failed_batch(batch_id).await?;
		},
	}
	tracing::debug!("executed query in {}s", now.elapsed().unwrap().as_secs());
	Ok(())
}
