use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tc_cli::{Benchmark, Query, Sender, Tc};
use time_primitives::{BatchId, BlockNumber, CCTPMessage, Hash, NetworkId, ShardId, TaskId};
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
		dest: NetworkId,
		src_addr: Option<String>,
		dest_addr: Option<String>,
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
		Command::MaxFeePerGas { network } => {
			let fee = tc.max_fee_per_gas(network).await?;
			tc.println(
				None,
				format!("EIP1559 max_fee_per_gas for network: {} is : {}", network, fee),
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
			let limit = tc.block_gas_limit(network).await?;
			tc.println(None, format!("Gas limit for block: {} is : {}", network, limit))
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
		Command::RegisterShards => {
			tc.register_online_shards().await?;
		},
		Command::RegisterRoutes => tc.register_all_routes().await?,
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
			let testers = tc.setup_test().await?;

			// collect shard batches
			let mut batches = HashSet::new();
			for shard in tc.shards().await? {
				if let Some(batch) = shard.batch_register {
					batches.insert(batch);
				}
			}
			// wait for shard batches to execute
			for batch in batches {
				let mut blocks = tc.finality_notification_stream();
				loop {
					if tc.is_batch_executed(batch).await? {
						break;
					}
					tracing::info!("waiting for batch {batch}");
					blocks.next().await.context("expected block")?;
				}
			}

			tc.assert_reimbursement().await?;
			let total_funds = tc.total_gateway_funds()?;
			let total_balance = tc.total_gateway_balance().await?;
			tc.println(
				None,
				format!("shard registration msgs cost {}$", total_funds - total_balance),
			)
			.await?;
			let _ = tc.exec_smoke(src, dest, &testers, vec![42]).await?;
			tc.assert_reimbursement().await?;
			let total_balance_after = tc.total_gateway_balance().await?;
			tc.println(
				None,
				format!("made {}$ of profit with msg", total_balance_after - total_balance),
			)
			.await?;
			anyhow::ensure!(total_balance_after >= total_balance);
		},
		Command::SmokeCctp { src, dest, src_addr, dest_addr } => {
			let testers = match (src_addr, dest_addr) {
				(Some(src_addr), Some(dest_addr)) => {
					let src_addr = tc.parse_address(Some(src), &src_addr)?;
					let dest_addr = tc.parse_address(Some(dest), &dest_addr)?;
					let mut testers = HashMap::new();
					testers.insert(src, (src_addr, 0));
					testers.insert(dest, (dest_addr, 0));
					testers
				},
				_ => tc.setup_test().await?,
			};
			let src_addr = testers.get(&src).context("missing tester")?.0;
			let dest_addr = testers.get(&dest).context("missing tester")?.0;
			tc.set_network_config(src, Some(src_addr)).await?;
			tc.set_network_config(dest, Some(dest_addr)).await?;
			let cctp_msg_data = "0000000000000000000000060000000000040CDD0000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001C7D4B196CB0C7B01D743FBC6116A902379C723800000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25000000000000000000000000000000000000000000000000000000000098968000000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25";
			let msg_data =
				hex::decode(cctp_msg_data).expect("Unable to create msg data from dummy cctp msg");
			let cctp_payload = CCTPMessage {
				attestation: vec![],
				message: msg_data,
				extra_data: [0u8; 32].to_vec(),
			};
			let msg = tc.exec_smoke(src, dest, &testers, cctp_payload.encode()).await?;
			let attested =
				CCTPMessage::from_bytes(&msg.bytes).map_err(|e| anyhow::anyhow!("{:?}", e))?;
			assert!(!attested.attestation.is_empty());
			assert!(attested.extra_data == cctp_payload.extra_data);
		},
		Command::Benchmark {
			num_messages_per_block,
			num_blocks,
		} => {
			let testers = tc.setup_test().await?;
			let mut benchmark =
				Benchmark::new(tc, testers, vec![42], num_messages_per_block, num_blocks);
			benchmark.add_routes().await?;
			benchmark.wait_for_sync().await?;
			benchmark.exec().await?;
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
