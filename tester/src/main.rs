use anyhow::Result;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::time::Duration;
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
	SetupGmp { shard_id: u64 },
	WatchTask { task_id: u64 },
	Basic,
	BatchTask { tasks: u64 },
	Gmp,
	TaskMigration,
	KeyRecovery { nodes: u8 },
}

#[tokio::main]
async fn main() -> Result<()> {
	let (params, cmd, contract) = args();

	let tester = Tester::new(params).await?;

	match cmd {
		TestCommand::FundWallet => {
			tester.faucet().await;
		},
		TestCommand::DeployContract => {
			tester.deploy(&contract, &[]).await?;
		},
		TestCommand::SetupGmp { shard_id } => {
			tester.setup_gmp(shard_id).await?;
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
	}

	Ok(())
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

	let shard_id = tester.get_shard_id().await;
	tester.setup_gmp(shard_id).await?;

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
	let shard_id = tester.get_shard_id().await;
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
