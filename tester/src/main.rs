use alloy_sol_types::SolCall;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use rosetta_config_ethereum::{AtBlock, GetTransactionCount, SubmitResult};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tc_subxt::ext::futures::future::join_all;
use tester::{
	format_duration, setup_gmp_with_contracts, sleep_or_abort, stats, test_setup, Network, Tester,
	VotingContract,
};
use tokio::time::Instant;

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_values = ["3;ws://ethereum:8545", "6;ws://astar:9944"])]
	network: Vec<Network>,
	#[arg(long, default_value = "/etc/alice")]
	timechain_keyfile: PathBuf,
	#[arg(long, default_value = "ws://validator:9944")]
	timechain_url: String,
	#[arg(long, default_value = "/etc/keyfile")]
	target_keyfile: PathBuf,
	#[arg(long, default_value = "/etc/contracts/gateway.sol/Gateway.json")]
	gateway_contract: PathBuf,
	#[arg(long, default_value = "/etc/contracts/test_contract.sol/VotingContract.json")]
	contract: PathBuf,
	#[clap(subcommand)]
	cmd: TestCommand,
}

#[derive(Parser, Debug)]
enum TestCommand {
	FundWallet,
	SetupGmp {
		/// Deploys and registers a new gateway contract even, replacing the existing one.
		#[clap(long, short = 'r', default_value_t = false)]
		redeploy: bool,
	},
	WatchTask {
		task_id: u64,
	},
	Basic,
	BatchTask {
		tasks: u64,
	},
	Gmp,
	GmpBenchmark {
		tasks: u64,
	},
	TaskMigration,
	KeyRecovery {
		nodes: u8,
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
	let args = Args::parse();
	let runtime = tester::subxt_client(&args.timechain_keyfile, &args.timechain_url).await?;
	let mut tester = Vec::with_capacity(args.network.len());
	for network in &args.network {
		tester.push(
			Tester::new(runtime.clone(), network, &args.target_keyfile, &args.gateway_contract)
				.await?,
		);
	}
	let contract = args.contract;

	match args.cmd {
		TestCommand::FundWallet => {
			tester[0].faucet().await;
		},
		TestCommand::SetupGmp { redeploy } => {
			tester[0].faucet().await;
			tester[0].setup_gmp(redeploy).await?;
		},
		TestCommand::WatchTask { task_id } => {
			tester[0].wait_for_task(task_id).await;
		},
		TestCommand::Basic => basic_test(&tester[0], &contract).await?,
		TestCommand::BatchTask { tasks } => {
			batch_test(&tester[0], &contract, tasks).await?;
		},
		TestCommand::Gmp => {
			gmp_test(&tester[0], &tester[1], &contract).await?;
		},
		TestCommand::TaskMigration => {
			task_migration_test(&tester[0], &contract).await?;
		},
		TestCommand::KeyRecovery { nodes } => {
			chronicle_restart_test(&tester[0], &contract, nodes).await?;
		},
		TestCommand::GmpBenchmark { tasks } => {
			gmp_benchmark(&tester[0], &tester[1], &contract, tasks).await?;
		},
	}
	Ok(())
}

async fn gmp_benchmark(
	src_tester: &Tester,
	dest_tester: &Tester,
	contract: &Path,
	number_of_calls: u64,
) -> Result<()> {
	let (src_contract, dest_contract) =
		setup_gmp_with_contracts(src_tester, dest_tester, contract, number_of_calls as u128)
			.await?;

	let mut calls = Vec::new();
	let bytes = hex::decode(&src_tester.wallet().account().address.strip_prefix("0x").unwrap())?;
	let mut address_bytes = [0u8; 20];
	address_bytes.copy_from_slice(&bytes[..20]);
	let nonce = src_tester
		.wallet()
		.query(GetTransactionCount {
			address: address_bytes.into(),
			block: AtBlock::Latest,
		})
		.await?;

	for i in 0..number_of_calls {
		let nonce = nonce + i;
		calls.push({
			src_tester.wallet().eth_send_call(
				src_contract,
				VotingContract::voteCall { _vote: true }.abi_encode(),
				0,
				Some(nonce),
				None,
			)
		});
	}

	let results: Vec<SubmitResult> = join_all(calls)
		.await
		.into_iter()
		.map(|result| result.unwrap())
		.collect::<Vec<_>>();

	let last_result = results.last().unwrap().receipt().unwrap().block_number;

	let target = stats(src_tester, src_contract, last_result).await?;
	println!("1: yes: {} no: {}", target.0, target.1);
	assert_eq!(target, (number_of_calls, 0));
	let start = Instant::now();
	loop {
		sleep_or_abort(Duration::from_secs(60)).await?;
		let current = stats(dest_tester, dest_contract, None).await?;
		println!(
			"2: yes: {} no: {}, time_since_start: {}",
			current.0,
			current.1,
			format_duration(start.elapsed())
		);
		if current == target {
			break;
		}
	}
	let duration = start.elapsed();
	println!("Time taken for {:?} gmp tasks is {:?}", number_of_calls, format_duration(duration));
	Ok(())
}

async fn basic_test(tester: &Tester, contract: &Path) -> Result<()> {
	let (_, contract_address, start_block) = test_setup(tester, contract).await?;

	let call = tester::create_evm_view_call(contract_address);
	tester.create_task_and_wait(call, start_block).await;

	let paid_call = tester::create_evm_call(contract_address);
	tester.create_task_and_wait(paid_call, start_block).await;

	Ok(())
}

async fn batch_test(tester: &Tester, contract: &Path, total_tasks: u64) -> Result<()> {
	let (_, contract_address, start_block) = test_setup(tester, contract).await?;

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

async fn gmp_test(src: &Tester, dest: &Tester, contract: &Path) -> Result<()> {
	let (src_contract, dest_contract) = setup_gmp_with_contracts(src, dest, contract, 1).await?;

	println!("submitting vote");
	// submit a vote on source contract (testing contract) which will emit a gmpcreated event on gateway contract
	let res = src
		.wallet()
		.eth_send_call(
			src_contract,
			VotingContract::voteCall { _vote: true }.abi_encode(),
			0,
			None,
			None,
		)
		.await?;
	let block = res.receipt().unwrap().block_number.unwrap();
	println!("submitted vote in block {block}, tx_hash: {:?}", res.tx_hash());

	let target = stats(src, src_contract, Some(block)).await?;
	println!("1: yes: {} no: {}", target.0, target.1);
	assert_eq!(target, (1, 0));
	loop {
		sleep_or_abort(Duration::from_secs(60)).await?;
		let current = stats(dest, dest_contract, None).await?;
		println!("2: yes: {} no: {}", current.0, current.1);
		if current == target {
			break;
		}
	}
	Ok(())
}

async fn task_migration_test(tester: &Tester, contract: &Path) -> Result<()> {
	let (_, contract_address, start_block) = test_setup(tester, contract).await?;

	let call = tester::create_evm_view_call(contract_address);
	let task_id = tester.create_task(call, start_block).await.unwrap();
	// wait for some cycles to run, Note: tasks are running in background
	sleep_or_abort(Duration::from_secs(60)).await?;

	// drop 2 nodes
	tester::drop_node("testnet-chronicle-eth1-1".to_string());
	tester::drop_node("testnet-chronicle-eth1-1".to_string());
	println!("dropped 2 nodes");

	// wait for some time
	let shard_id = tester.get_shard_id().await?.unwrap();
	while tester.is_shard_online(shard_id).await {
		println!("Waiting for shard offline");
		sleep_or_abort(Duration::from_secs(10)).await?;
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
	let (_, contract_address, start_block) = test_setup(tester, contract).await?;

	let call = tester::create_evm_view_call(contract_address);
	let task_id = tester.create_task(call, start_block).await?;

	// wait for some cycles to run, Note: tasks are running in background
	for i in 1..nodes_to_restart + 1 {
		println!("waiting for 1 min");
		sleep_or_abort(Duration::from_secs(60)).await?;
		println!("restarting node {}", i);
		tester::restart_node(format!("testnet-chronicle-eth{}-1", i));
	}

	println!("waiting for 20 secs to let node recover completely");
	sleep_or_abort(Duration::from_secs(20)).await?;

	// watch task
	tester.wait_for_task(task_id).await;

	Ok(())
}
