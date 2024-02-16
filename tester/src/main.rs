use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tc_subxt::ext::futures::future::join_all;
use tester::{Tester, TesterParams};
use time_primitives::NetworkId;

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_value = "3")]
	network_id: NetworkId,
	#[arg(long, default_value = "/etc/alice")]
	timechain_keyfile: PathBuf,
	#[arg(long, default_value = "ws://validator:9944")]
	timechain_url: String,
	#[arg(long, default_value = "/etc/keyfile")]
	target_keyfile: PathBuf,
	#[arg(long, default_value = "ws://ethereum:8545")]
	target_url: String,
	#[arg(long, default_value = "/etc/gateway.sol")]
	gateway_contract: PathBuf,
	#[arg(long, default_value = "/etc/test_contract.sol")]
	contract: PathBuf,
	#[clap(subcommand)]
	cmd: TestCommand,
}

fn args() -> (TesterParams, TestCommand, PathBuf) {
	let args = Args::parse();
	let params = TesterParams {
		network_id: args.network_id,
		timechain_keyfile: args.timechain_keyfile,
		timechain_url: args.timechain_url,
		target_keyfile: args.target_keyfile,
		target_url: args.target_url,
		gateway_contract: args.gateway_contract,
	};
	(params, args.cmd, args.contract)
}

#[derive(Parser, Debug)]
enum TestCommand {
	FundWallet,
	DeployContract,
	SetupGmp,
	WatchTask {
		task_id: u64,
	},
	Basic,
	BatchTask {
		tasks: u64,
	},
	Gmp,
	TaskMigration,
	KeyRecovery {
		nodes: u8,
	},
	///
	/// # Arguments:
	/// * `env`: on which env to run local/staging.
	/// * `tasks`: number of tasks to register at once.
	/// * `cycle`: number of times to register task at once.
	/// * `block_timeout`: total number of blocks after which stop watching
	LatencyCheck {
		env: Environment,
		tasks: u64,
		cycle: u64,
		block_timeout: u64,
	},
}

#[derive(ValueEnum, Debug, Clone)]
enum Environment {
	Local,
	Staging,
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();
	let (params, cmd, contract) = args();

	let tester = Tester::new(params).await?;

	match cmd {
		TestCommand::FundWallet => {
			tester.faucet().await;
		},
		TestCommand::DeployContract => {
			tester.deploy(&contract, &[]).await?;
		},
		TestCommand::SetupGmp => {
			tester.setup_gmp().await?;
		},
		TestCommand::WatchTask { task_id } => {
			tester.wait_for_task(task_id).await;
		},
		TestCommand::Basic => basic_test(&tester, &contract).await?,
		TestCommand::BatchTask { tasks } => {
			batch_test(&tester, &contract, tasks).await?;
		},
		TestCommand::Gmp => {
			gmp_test(&tester, &contract).await?;
		},
		TestCommand::TaskMigration => {
			task_migration_test(&tester, &contract).await?;
		},
		TestCommand::KeyRecovery { nodes } => {
			chronicle_restart_test(&tester, &contract, nodes).await?;
		},
		TestCommand::LatencyCheck {
			env,
			tasks,
			cycle,
			block_timeout,
		} => {
			latency_checker(&tester, env, tasks, cycle, block_timeout, &contract).await?;
		},
	}
	Ok(())
}

async fn latency_checker(
	tester: &Tester,
	env: Environment,
	tasks: u64,
	rounds: u64,
	block_timeout: u64,
	contract: &Path,
) -> Result<()> {
	let mut overall_latencies = 0.0;
	let mut overall_throughput = 0.0;

	let mut rounds_future = Vec::new();
	for c in 0..rounds {
		let data = latency_cycle(tester, env.clone(), tasks, block_timeout, c, contract);
		rounds_future.push(data);
		// wait approximate block time;
		tokio::time::sleep(Duration::from_secs(6)).await;
	}

	let cycles_data = join_all(rounds_future).await;

	for data in cycles_data {
		if let Ok((cycle_latency, cycle_throughput)) = data {
			overall_latencies += cycle_latency;
			overall_throughput += cycle_throughput;
		}
	}

	let average_latencies = overall_latencies / (rounds as f32);
	println!(
		"Average latency for round(s) {:?} of total {:?} tasks each is {:?} blocks per task",
		rounds, tasks, average_latencies
	);
	let average_throughput = overall_throughput / (rounds as f32);
	println!(
		"Average Throughput for round(s) {:?} of total of {:?} tasks is {:?} tasks per block",
		rounds, tasks, average_throughput
	);
	Ok(())
}
async fn latency_cycle(
	tester: &Tester,
	env: Environment,
	total_tasks: u64,
	block_timeout: u64,
	round_num: u64,
	contract: &Path,
) -> Result<(f32, f32)> {
	let (contract_address, start_block) = match env {
		Environment::Local => {
			tester.faucet().await;
			tester.setup_gmp().await?;
			tester.deploy(contract, &[]).await?
		},
		Environment::Staging => ("0xb77791b3e38158475216dd4c0e2143b858188ba6".to_string(), 0),
	};

	let mut registerations = vec![];
	for i in 0..total_tasks {
		let mut salt = [0u8; 32];
		let randomness = i.to_ne_bytes();
		salt[..8].copy_from_slice(&randomness);
		let send_msg =
			tester::create_send_msg_call(contract_address.clone(), "vote_yes()", salt, 10000000);
		registerations.push(tester.create_task(send_msg, start_block));
	}

	let mut task_ids: Vec<u64> = join_all(registerations)
		.await
		.into_iter()
		.map(|result| result.unwrap())
		.collect();

	let starting_block = tester.get_latest_block().await?;

	let mut finished_tasks = vec![];
	loop {
		let current_block = tester.get_latest_block().await?;
		let status: Vec<_> = task_ids
			.iter()
			.map(|&id| async move { (id, tester.is_task_finished(id).await) })
			.collect();
		let results: Vec<(u64, bool)> = join_all(status).await;

		for (task_id, is_completed) in results {
			if is_completed {
				println!("task_id {:?} completed", task_id);
				finished_tasks.push(task_id);
				let index = task_ids.iter().position(|x| *x == task_id).unwrap();
				task_ids.remove(index);
			}
		}

		if (current_block - starting_block) > block_timeout
			|| finished_tasks.len() == total_tasks as usize
		{
			break;
		}

		tokio::time::sleep(Duration::from_secs(6)).await;
	}
	let ending_block = tester.get_latest_block().await?;

	if task_ids.is_empty() {
		let round_num = round_num + 1;
		assert!(finished_tasks.len() == total_tasks as usize);
		let total_block_time = ending_block - starting_block;
		let latency = total_block_time as f32 / total_tasks as f32;

		println!(
			"Total block time taken by round {:?} of {:?} tasks is {:?} blocks",
			round_num, total_tasks, total_block_time
		);
		let throughput = total_tasks as f32 / total_block_time as f32;
		println!("Latency for {:?} round is {:?} blocks per task", round_num, latency);
		println!("Throughput for {:?} round is {:?} tasks per block", round_num, throughput);
		return Ok((latency, throughput));
	} else {
		println!(
			"Error computing latency, completed tasks are {:?}/{:?}",
			finished_tasks.len(),
			total_tasks
		);
		return Ok((0.0, 0.0));
	};
	// returns (latency, throughput)
}

async fn basic_test(tester: &Tester, contract: &Path) -> Result<()> {
	tester.faucet().await;
	let (contract_address, start_block) = tester.deploy(contract, &[]).await?;

	let call = tester::create_evm_view_call(contract_address.clone());
	tester.create_task_and_wait(call, start_block).await;

	let paid_call = tester::create_evm_call(contract_address);
	tester.create_task_and_wait(paid_call, start_block).await;

	Ok(())
}

async fn batch_test(tester: &Tester, contract: &Path, total_tasks: u64) -> Result<()> {
	tester.faucet().await;
	let (contract_address, start_block) = tester.deploy(contract, &[]).await?;

	let mut task_ids = vec![];
	let call = tester::create_evm_view_call(contract_address);
	for _ in 0..total_tasks {
		let task_id = tester.create_task(call.clone(), start_block).await?;
		task_ids.push(task_id);
	}
	for task_id in task_ids {
		tester.wait_for_task(task_id).await;
	}

	Ok(())
}

async fn gmp_test(tester: &Tester, contract: &Path) -> Result<()> {
	tester.faucet().await;
	tester.setup_gmp().await?;

	let (contract_address, start_block) = tester.deploy(contract, &[]).await?;

	let send_msg = tester::create_send_msg_call(contract_address, "vote_yes()", [1; 32], 10000000);

	tester.create_task_and_wait(send_msg, start_block).await;
	Ok(())
}

async fn task_migration_test(tester: &Tester, contract: &Path) -> Result<()> {
	tester.faucet().await;
	let (contract_address, start_block) = tester.deploy(contract, &[]).await?;

	let call = tester::create_evm_view_call(contract_address);
	let task_id = tester.create_task(call, start_block).await.unwrap();
	// wait for some cycles to run, Note: tasks are running in background
	tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

	// drop 2 nodes
	tester::drop_node("testnet-chronicle-eth1-1".to_string());
	tester::drop_node("testnet-chronicle-eth1-1".to_string());
	println!("dropped 2 nodes");

	// wait for some time
	let shard_id = tester.get_shard_id().await?.unwrap();
	while tester.is_shard_online(shard_id).await {
		println!("Waiting for shard offline");
		tokio::time::sleep(tokio::time::Duration::from_secs(50)).await;
	}
	println!("Shard is offline, starting nodes");

	// start nodes again
	tester::start_node("testnet-chronicle-eth1-1".to_string());
	tester::start_node("testnet-chronicle-eth1-1".to_string());

	// watch task
	tester.wait_for_task(task_id).await;

	Ok(())
}

async fn chronicle_restart_test(
	tester: &Tester,
	contract: &Path,
	nodes_to_restart: u8,
) -> Result<()> {
	tester.faucet().await;
	let (contract_address, start_block) = tester.deploy(contract, &[]).await?;

	let call = tester::create_evm_view_call(contract_address);
	let task_id = tester.create_task(call, start_block).await?;

	// wait for some cycles to run, Note: tasks are running in background
	for i in 1..nodes_to_restart + 1 {
		println!("waiting for 1 min");
		tokio::time::sleep(Duration::from_secs(60)).await;
		println!("restarting node {}", i);
		tester::restart_node(format!("testnet-chronicle-eth{}-1", i));
	}

	println!("waiting for 20 secs to let node recover completely");
	tokio::time::sleep(Duration::from_secs(20)).await;

	// watch task
	tester.wait_for_task(task_id).await;

	Ok(())
}
