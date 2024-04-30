use alloy_sol_types::SolCall;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use rosetta_config_ethereum::{AtBlock, GetTransactionCount, SubmitResult};
use std::path::{Path, PathBuf};
use std::time::Duration;
use sysinfo::System;
use tc_subxt::ext::futures::StreamExt;
use tc_subxt::timechain_runtime::tasks::events;
use tc_subxt::SubxtClient;
use tester::{
	format_duration, setup_funds_if_needed, setup_gmp_with_contracts, sleep_or_abort, stats,
	test_setup, wait_for_gmp_calls, GmpBenchState, Network, Tester, VotingContract,
};
use tokio::time::{interval_at, Instant};

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
	cmd: Command,
}

#[derive(Parser, Debug)]
enum Command {
	FundWallet,
	SetupGmp {
		/// Deploys and registers a new gateway contract even, replacing the existing one.
		#[clap(long, short = 'r', default_value_t = false)]
		redeploy: bool,
	},
	SetShardConfig {
		shard_size: u16,
		shard_threshold: u16,
	},
	WatchTask {
		task_id: u64,
	},
	GmpBenchmark {
		tasks: u64,
		test_contract_addresses: Vec<String>,
	},
	#[clap(subcommand)]
	Test(Test),
}

#[derive(Parser, Debug)]
enum Test {
	Basic,
	Batch { tasks: u64 },
	Gmp,
	Migration,
	Restart,
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
		Command::FundWallet => {
			tester[0].faucet().await;
		},
		Command::SetupGmp { redeploy } => {
			tester[0].faucet().await;
			tester[0].setup_gmp(redeploy).await?;
		},
		Command::SetShardConfig { shard_size, shard_threshold } => {
			tester[0].set_shard_config(shard_size, shard_threshold).await?;
		},
		Command::WatchTask { task_id } => {
			tester[0].wait_for_task(task_id).await;
		},
		Command::GmpBenchmark { tasks, test_contract_addresses } => {
			let contracts = if test_contract_addresses.len() >= 2 {
				Some((test_contract_addresses[0].clone(), test_contract_addresses[1].clone()))
			} else {
				None
			};
			gmp_benchmark(&args.timechain_url, &tester[0], &tester[1], &contract, tasks, contracts)
				.await?;
		},
		Command::Test(Test::Basic) => basic_test(&tester[0], &contract).await?,
		Command::Test(Test::Batch { tasks }) => {
			batch_test(&tester[0], &contract, tasks).await?;
		},
		Command::Test(Test::Gmp) => {
			gmp_test(&tester[0], &tester[1], &contract).await?;
		},
		Command::Test(Test::Migration) => {
			task_migration_test(&tester[0], &contract).await?;
		},
		Command::Test(Test::Restart) => {
			chronicle_restart_test(&tester[0], &contract).await?;
		},
	}
	Ok(())
}

async fn gmp_benchmark(
	timechain_url: &str,
	src_tester: &Tester,
	dest_tester: &Tester,
	contract: &Path,
	number_of_calls: u64,
	test_contracts: Option<(String, String)>,
) -> Result<()> {
	println!("Running gmp benchmark for {:?} GMP calls", number_of_calls);

	let mut sys = System::new_all();
	let mut memory_usage = vec![];
	let mut cpu_usage = vec![];

	// Initialized it to get events from timechain
	// SubxtClient client doesnt support exporting client to outer space
	// doesnt want to modify it without asking David.
	let subxt_client = SubxtClient::get_client(timechain_url).await?;

	// gmp_bench_state to do analysis on data in the end
	let mut bench_state = GmpBenchState::new(number_of_calls);

	// check if deployed test contracts are already provided
	let (src_contract, dest_contract, deposit_amount) = if let Some(test_contracts) = test_contracts
	{
		// if contracts are already deployed check the funds and add more funds in gateway contract if needed
		setup_funds_if_needed(test_contracts, src_tester, dest_tester, number_of_calls as u128)
			.await?
	} else {
		// if contracts are not provided deploy contracts and fund gmp contract
		setup_gmp_with_contracts(src_tester, dest_tester, contract, number_of_calls as u128).await?
	};

	bench_state.set_deposit(deposit_amount);

	// get contract stats of src contract
	let start_stats = stats(src_tester, src_contract, None).await?;
	// get contract stats of destination contract
	let dest_stats = stats(src_tester, src_contract, None).await?;
	println!("stats in start: {:?}", start_stats);

	//get nonce of the caller to manage explicitly
	let bytes = hex::decode(src_tester.wallet().account().address.strip_prefix("0x").unwrap())?;
	let mut address_bytes = [0u8; 20];
	address_bytes.copy_from_slice(&bytes[..20]);
	let nonce = src_tester
		.wallet()
		.query(GetTransactionCount {
			address: address_bytes.into(),
			block: AtBlock::Latest,
		})
		.await?;

	// make list of contract calls to initiate gmp tasks
	let mut calls = Vec::new();
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

	// wait for calls in chunks
	let results: Vec<SubmitResult> = wait_for_gmp_calls(calls, number_of_calls, 25).await?;
	let all_gmp_blocks = results
		.iter()
		.map(|item| item.receipt().unwrap().block_number.unwrap())
		.collect::<Vec<_>>();
	println!("tx hash for sample gmp call {:?}", results.first().unwrap().tx_hash());
	println!("tx hash for block {:?}", results.first().unwrap().receipt().unwrap().block_number);
	let gas_amount_used = results
		.iter()
		.map(|result| {
			let receipt = result.receipt().unwrap();
			let gas_price = u128::try_from(receipt.effective_gas_price.unwrap()).unwrap();
			let gas_used = u128::try_from(receipt.gas_used.unwrap_or_default()).unwrap();
			gas_price.saturating_mul(gas_used)
		})
		.collect::<Vec<_>>();

	// total gas fee for src_contract call
	bench_state.insert_src_gas(gas_amount_used);

	// start the timer for gmp execution
	bench_state.start();

	// Get last block result of contract stats
	let last_result = results.last().unwrap().receipt().unwrap().block_number;

	// get src contract result
	let src_stats = stats(src_tester, src_contract, last_result).await?;
	println!("1: yes: {} no: {}", src_stats.0, src_stats.1);
	assert_eq!(src_stats, (number_of_calls + start_stats.0, start_stats.1));

	let mut block_stream = src_tester.finality_block_stream().await;
	let mut one_min_tick = interval_at(Instant::now(), Duration::from_secs(60));

	// loop to listen for task change and stats events from destination chain
	loop {
		let tasks_in_bench = bench_state.task_ids();
		tokio::select! {
			block = block_stream.next() => {
				if let Some((block_hash, _)) = block {
					let events = subxt_client.events().at(block_hash).await?;
					let task_inserted_events = events.find::<events::TaskCreated>();
					for task in task_inserted_events.flatten() {
						let task_id = task.0;
						let task_details = src_tester.get_task(task_id).await;
						match task_details.function {
							time_primitives::Function::SendMessage { msg } => {
								if msg.dest == dest_contract {
									// GMP task found matching destination contract
									bench_state.add_task(task_id);
								}
							},
							time_primitives::Function::ReadMessages { batch_size } => {
								let start_block = task_details.start - (batch_size.get() - 1);
								for item in start_block..task_details.start + 1 {
									let count = all_gmp_blocks.iter().filter(|block| *block == &item).count();
									if count > 0 {
										println!("Timechain received tasks for {:?} gmp receive task in {:?}", count, bench_state.current_duration());
									}
								}
							},
							_ => {},
						}
					}

					// finish tasks
					let task_result_submitted = events.find::<events::TaskResult>();
					for task_result in task_result_submitted.flatten() {
						let task_id = task_result.0;
						if tasks_in_bench.contains(&task_id) {
							bench_state.finish_task(task_id);
						}
					}
					// update task phase
					bench_state.sync_phase(src_tester).await;
				}
			}
			_ = one_min_tick.tick() => {
				let current = stats(dest_tester, dest_contract, None).await?;
				if current != (dest_stats.0 + number_of_calls, dest_stats.1) {
					println!(
						"2: yes: {} no: {}, time_since_start: {}",
						current.0,
						current.1,
						format_duration(bench_state.current_duration())
					);
				} else {
					println!("contract updated, waiting for task to complete");
				}

				// compute memory usage
				sys.refresh_memory();
				let total_memory = sys.total_memory() as f64;
				let used_memory = sys.used_memory() as f64;
				let memory_usage_percent = (used_memory / total_memory) * 100.0;
				memory_usage.push(memory_usage_percent);

				// compute cpu usage
				sys.refresh_cpu();
				let cpu_count = sys.cpus().len() as f64;
				let total_cpu_usage: f64 = sys.cpus().iter()
						.map(|cpu| cpu.cpu_usage() as f64)
						.fold(0.0, |acc, x| acc + x);
				let average_cpu_usage = if cpu_count == 0.0  {
					0.0
				} else {
					total_cpu_usage / cpu_count
				};

				cpu_usage.push(average_cpu_usage);

				if bench_state.get_finished_tasks() == number_of_calls as usize {
					break;
				} else {
					println!("task_ids: {:?}, completed: {:?}", bench_state.task_ids(), bench_state.get_finished_tasks());
				}
			}
			_ = tokio::signal::ctrl_c() => {
				println!("aborting...");
				anyhow::bail!("abort");
			}
		}
	}
	bench_state.finish();
	bench_state.print_analysis();
	println!(
		"Average memory consumed is {:.2}%",
		memory_usage.iter().sum::<f64>() / memory_usage.len() as f64
	);
	println!("Average cpu usage is {:.2}%", cpu_usage.iter().sum::<f64>() / cpu_usage.len() as f64);
	Ok(())
}

async fn basic_test(tester: &Tester, contract: &Path) -> Result<()> {
	let (_, contract_address, start_block) = test_setup(tester, contract, 1, 1).await?;

	let call = tester::create_evm_view_call(contract_address);
	tester.create_task_and_wait(call, start_block).await;

	let paid_call = tester::create_evm_call(contract_address);
	tester.create_task_and_wait(paid_call, start_block).await;

	Ok(())
}

async fn batch_test(tester: &Tester, contract: &Path, total_tasks: u64) -> Result<()> {
	let (_, contract_address, start_block) = test_setup(tester, contract, 1, 1).await?;

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
	src.set_shard_config(1, 1).await?;
	let (src_contract, dest_contract, _) = setup_gmp_with_contracts(src, dest, contract, 1).await?;

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
	let (_, contract_address, start_block) = test_setup(tester, contract, 1, 1).await?;

	let call = tester::create_evm_view_call(contract_address);
	let task_id = tester.create_task(call, start_block).await.unwrap();
	// wait for some cycles to run, Note: tasks are running in background
	sleep_or_abort(Duration::from_secs(60)).await?;

	// drop 2 nodes
	tester::stop_node("testnet-chronicle-eth1-1".to_string());
	tester::stop_node("testnet-chronicle-eth1-1".to_string());
	println!("dropped 2 nodes");

	// wait for some time
	let shard_id = tester.get_shard_id().await?.unwrap();
	while tester.is_shard_online(shard_id).await? {
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

async fn chronicle_restart_test(tester: &Tester, contract: &Path) -> Result<()> {
	let (_, contract_address, start_block) = test_setup(tester, contract, 3, 2).await?;
	let shard_size = tester.shard_size().await?;
	let threshold = tester.shard_threshold().await?;

	for i in 0..shard_size {
		if i < threshold {
			tester::restart_node(format!("testnet-chronicle-eth{}-1", i + 1));
		} else {
			tester::stop_node(format!("testnet-chronicle-eth{}-1", i + 1));
		}
	}

	let call = tester::create_evm_view_call(contract_address);
	tester.create_task_and_wait(call, start_block).await;

	Ok(())
}
