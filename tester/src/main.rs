use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use num_bigint::BigUint;
use rosetta_config_ethereum::{AtBlock, CallResult, GetTransactionCount, SubmitResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use sysinfo::System;
use tc_subxt::ext::futures::{FutureExt, StreamExt};
use tc_subxt::{events, MetadataVariant, SubxtClient};
use tester::sol::{Gateway, Network, VotingContract};
use tester::{
	format_duration, rational_to_ufloat, setup_gmp_with_contracts, sleep_or_abort, stats,
	test_setup, wait_for_gmp_calls, ChainNetwork, EthContractAddress, GmpBenchState, Tester,
};
use time_primitives::{Payload, ShardId};
use tokio::time::{interval_at, Instant};

// 0xD3e34B4a2530956f9eD2D56e3C6508B7bBa3aC84 tester wallet key
// 0x56AEe94c0022F866f7f15BeB730B987826AfA4C5 keyfile1
// 0x64AC191E26b66564bfda3249de27C9a8A96F9981 keyfile2
// 0x1Be6ACA05B9e3E28Cb8ED04B99C9B989D1342eF4 keyfile3
const CHRONICLE_KEYFILES: [&str; 3] = ["/etc/keyfile1", "/etc/keyfile2", "/etc/keyfile3"];

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_values = ["3;ws://ethereum:8545", "6;ws://astar:9944"])]
	network: Vec<ChainNetwork>,
	#[arg(long)]
	timechain_metadata: Option<MetadataVariant>,
	#[arg(long, default_value = "/etc/alice")]
	timechain_keyfile: PathBuf,
	#[arg(long, default_value = "ws://validator:9944")]
	timechain_url: String,
	#[arg(long, default_value = "/etc/keyfile")]
	target_keyfile: PathBuf,
	#[arg(long, default_value = "/etc/contracts/GatewayProxy.sol/GatewayProxy.json")]
	proxy_gateway_contract: PathBuf,
	#[arg(long, default_value = "/etc/contracts/Gateway.sol/Gateway.json")]
	gateway_contract: PathBuf,
	#[arg(long, default_value = "/etc/contracts/test_contract.sol/VotingContract.json")]
	contract: PathBuf,
	#[clap(subcommand)]
	cmd: Command,
}

#[derive(Parser, Debug)]
enum Command {
	FundWallet,
	GatewayUpgrade {
		proxy_address: String,
	},
	GatewaySetAdmin {
		proxy_address: String,
		admin: String,
	},
	GatewayAddShards {
		shard_ids: Vec<ShardId>,
	},
	GmpBenchmark {
		tasks: u64,
		test_contract_addresses: Vec<String>,
	},
	RegisterGmpShard {
		shard_id: ShardId,
		#[clap(default_value = "/etc/gmp_signer")]
		keyfile: PathBuf,
	},
	RegisterNetwork {
		chain_name: String,
		chain_network: String,
	},
	SetupGmp {
		/// Deploys and registers a new gateway contract even, replacing the existing one.
		#[clap(long, short = 'r', default_value_t = false)]
		redeploy: bool,
		/// optional mnemonic for sudo key
		#[arg(long)]
		keyfile: Option<PathBuf>,
	},
	SetShardConfig {
		shard_size: u16,
		shard_threshold: u16,
	},
	SetNetworkInfo {
		num: BigUint,
		den: BigUint,
	},
	#[clap(subcommand)]
	Test(Test),
	WatchTask {
		task_id: u64,
	},
}

#[derive(Parser, Debug)]
enum Test {
	Basic,
	Batch {
		tasks: u64,
	},
	Gmp,
	ChronicleMinFundCheck,
	Migration,
	Restart,
	ChroniclePaymentRefund {
		#[clap(default_value_t = 10000000000000000)]
		src_deposit: u128,
		#[clap(default_value_t = 10000000000000000)]
		dest_deposit: u128,
	},
}

#[derive(ValueEnum, Debug, Clone)]
enum Environment {
	Local,
	Staging,
}

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt::init();
	let args = Args::parse();
	let runtime = tester::subxt_client(
		&args.timechain_keyfile,
		args.timechain_metadata.unwrap_or_default(),
		&args.timechain_url,
	)
	.await
	.unwrap();
	let mut testers = Vec::with_capacity(args.network.len());
	let mut chronicles = vec![];
	for network in &args.network {
		testers.push(
			Tester::new(
				runtime.clone(),
				network,
				&args.target_keyfile,
				&args.gateway_contract,
				&args.proxy_gateway_contract,
			)
			.await
			.unwrap(),
		);

		// fund chronicle faucets for testing
		for item in CHRONICLE_KEYFILES {
			let chronicle_tester = Tester::new(
				runtime.clone(),
				network,
				&PathBuf::from(item),
				&PathBuf::new(),
				&PathBuf::new(),
			)
			.await
			.unwrap();
			chronicle_tester.faucet().await;
			chronicles.push(chronicle_tester);
		}
	}
	let contract = args.contract;

	match args.cmd {
		Command::FundWallet => {
			testers[0].faucet().await;
		},
		Command::SetupGmp { redeploy, keyfile } => {
			let mut network_map = HashMap::new();
			for tester in &testers {
				tester.faucet().await;
				let network_id = tester.network_id();
				let proxy_addr = tester.get_proxy_addr().await.unwrap();
				network_map.entry(network_id).or_insert_with(|| Network {
					id: network_id,
					gateway: proxy_addr,
				});
			}
			let networks: Vec<Network> = network_map.into_values().collect();
			for tester in &testers {
				tester.setup_gmp(redeploy, keyfile.clone(), networks.clone()).await.unwrap();
			}
		},
		Command::RegisterGmpShard { shard_id, keyfile } => {
			testers[0].register_shard_on_gateway(shard_id, keyfile).await.unwrap();
		},
		Command::RegisterNetwork { chain_name, chain_network } => {
			testers[0].register_network(chain_name, chain_network).await.unwrap();
		},
		Command::SetShardConfig { shard_size, shard_threshold } => {
			testers[0].set_shard_config(shard_size, shard_threshold).await.unwrap();
		},
		Command::SetNetworkInfo { num, den } => {
			rational_to_ufloat(num, den);
		},
		Command::WatchTask { task_id } => {
			testers[0].wait_for_task(task_id).await;
		},
		Command::GatewayUpgrade { proxy_address } => {
			let proxy_address = Address::from_str(&proxy_address).unwrap();
			testers[0].gateway_update(proxy_address).await.unwrap();
		},
		Command::GatewaySetAdmin { proxy_address, admin } => {
			let proxy_address = Address::from_str(&proxy_address).unwrap();
			let admin_address = Address::from_str(&admin).unwrap();
			testers[0].gateway_set_admin(proxy_address, admin_address).await.unwrap();
		},
		Command::GatewayAddShards { shard_ids } => {
			testers[0].gateway_add_shards(shard_ids).await.unwrap();
		},
		Command::GmpBenchmark { tasks, test_contract_addresses } => {
			let contracts = if test_contract_addresses.len() >= 2 {
				Some((test_contract_addresses[0].clone(), test_contract_addresses[1].clone()))
			} else {
				None
			};
			gmp_benchmark(
				&args.timechain_url,
				&testers[0],
				&testers[1],
				&contract,
				tasks,
				contracts,
			)
			.await
			.unwrap();
		},
		Command::Test(Test::Basic) => basic_test(&testers[0], &contract).await.unwrap(),
		Command::Test(Test::Batch { tasks }) => {
			batch_test(&testers[0], &contract, tasks).await.unwrap();
		},
		Command::Test(Test::ChroniclePaymentRefund { src_deposit, dest_deposit }) => {
			// This test is meant to run on local
			// Make sure GMP is already setup otherwise this test will fail
			let starting_balance = chronicles[0].wallet().balance().await.unwrap();
			gmp_test(&testers[0], &testers[1], &contract, Some((src_deposit, dest_deposit)))
				.await
				.unwrap();
			let ending_balance = chronicles[0].wallet().balance().await.unwrap();
			assert!(starting_balance <= ending_balance);
		},
		Command::Test(Test::ChronicleMinFundCheck) => {
			// "This test is only available in local setup"
			gmp_funds_check(&testers[0], &testers[1], &contract, &chronicles, &args.timechain_url)
				.await
				.unwrap();
		},
		Command::Test(Test::Gmp) => {
			gmp_test(&testers[0], &testers[1], &contract, None).await.unwrap();
		},
		Command::Test(Test::Migration) => {
			task_migration_test(&testers[0], &contract).await.unwrap();
		},
		Command::Test(Test::Restart) => {
			chronicle_restart_test(&testers[0], &contract).await.unwrap();
		},
	}
	println!("Command executed");
	std::process::exit(0);
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
	let mut is_contract_updated = false;

	// Initialized it to get events from timechain
	// SubxtClient client doesnt support exporting client to outer space
	// doesnt want to modify it without asking David.
	let subxt_client = SubxtClient::get_client(timechain_url).await?;

	// gmp_bench_state to do analysis on data in the end
	let mut bench_state = GmpBenchState::new(number_of_calls);

	// check if deployed test contracts are already provided
	let (src_contract, dest_contract) = if let Some(contracts) = test_contracts {
		let mut src_contract: EthContractAddress = [0; 20];
		let mut dest_contract: EthContractAddress = [0; 20];
		src_contract.copy_from_slice(
			&hex::decode(contracts.0.strip_prefix("0x").unwrap_or(&contracts.0)).unwrap()[..20],
		);
		dest_contract.copy_from_slice(
			&hex::decode(contracts.1.strip_prefix("0x").unwrap_or(&contracts.1)).unwrap()[..20],
		);
		(src_contract, dest_contract)
	} else {
		setup_gmp_with_contracts(src_tester, dest_tester, contract).await?
	};

	// get contract stats of src contract
	let start_stats = stats(src_tester, src_contract, None).await?;
	// get contract stats of destination contract
	let dest_stats = stats(dest_tester, dest_contract, None).await?;
	println!("stats in start: {:?}", start_stats);
	let src_proxy = src_tester.geteway().await?.context("Gateway not found")?;
	let voting_call = VotingContract::voteCall { _vote: true }.abi_encode();
	let msg_size = U256::from_str_radix(&voting_call.len().to_string(), 16).unwrap();

	println!("sending estimateMsg cost to: {:?}", src_proxy);
	// estimate message_cost
	let result = src_tester
		.wallet()
		.eth_send_call(
			src_proxy,
			Gateway::estimateMessageCostCall {
				networkid: dest_tester.network_id(),
				messageSize: msg_size,
				gasLimit: U256::from(100_000),
			}
			.abi_encode(),
			0,
			None,
			None,
		)
		.await?;

	let SubmitResult::Executed { result, .. } = result else { anyhow::bail!("{:?}", result) };
	let CallResult::Success(data) = result else { anyhow::bail!("failed parsing {:?}", result) };
	let msg_cost: u128 = Gateway::estimateMessageCostCall::abi_decode_returns(&data, true)?
		._0
		.try_into()
		.unwrap();

	// get nonce of the caller to manage explicitly
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
				voting_call.clone(),
				msg_cost,
				Some(nonce),
				None,
			)
		});
	}

	println!("single message cost: {:?}", msg_cost);
	println!("total message cost: {:?}", (msg_cost * number_of_calls as u128));

	// block stream of timechain
	let mut block_stream = src_tester.finality_block_stream().await;
	let mut one_min_tick = interval_at(Instant::now(), Duration::from_secs(60));

	let mut gmp_task = Box::pin(wait_for_gmp_calls(calls, number_of_calls, 25)).fuse();
	let mut all_gmp_blocks: Vec<u64> = vec![];

	// loop to listen for task change and stats events from destination chain
	loop {
		tokio::select! {
			// wait for gmp calls to be sent to src contract
			result = &mut gmp_task => {
				let result = result.unwrap();
				let blocks = result
					.iter()
					.map(|item| item.receipt().expect("Transaction receipt not found").block_number.expect("Transaction not finalized"))
					.collect::<Vec<_>>();
				all_gmp_blocks.extend(blocks);
				println!("tx hash for first gmp call {:?}", result.first().expect("Gmp calls are empty").tx_hash());
				println!(
					"tx block for first gmp call {:?}",
					result.first().unwrap().receipt().unwrap().block_number
				);

				let gas_amount_used = result
					.iter()
					.map(|result| {
						let receipt = result.receipt().expect("Transaction receipt not found");
						let gas_price = u128::try_from(receipt.effective_gas_price.unwrap()).expect("Unable to parse gas price");
						let gas_used = u128::try_from(receipt.gas_used.unwrap_or_default()).expect("Unable to parse gas used");
						gas_price.saturating_mul(gas_used)
					})
					.collect::<Vec<_>>();

				// total gas fee for src_contract call
				bench_state.insert_src_gas(gas_amount_used);

				// Get last block result of contract stats
				let last_result = result.last().unwrap().receipt().unwrap().block_number;
				let src_stats = stats(src_tester, src_contract, last_result).await?;
				println!("1: yes: {} no: {}", src_stats.0, src_stats.1);
				assert_eq!(src_stats, (number_of_calls + start_stats.0, start_stats.1));

				// start the timer for gmp execution
				bench_state.start();
			}
			block = block_stream.next() => {
				if let Some((block_hash, _)) = block {
					let events = subxt_client.events().at(block_hash).await?;
					let task_inserted_events = events.find::<events::TaskCreated>();
					for task in task_inserted_events.flatten() {
						let task_id = task.0;
						let task_details = src_tester.get_task(task_id).await;
						match task_details.function {
							// send message task inserted verify if is for our testing contract
							time_primitives::Function::SendMessage { msg } => {
								if msg.dest == dest_contract {
									// GMP task found matching destination contract
									bench_state.add_task(task_id);
								}
							},
							// insert read messages fetched
							time_primitives::Function::ReadMessages { batch_size } => {
								let start_block = task_details.start - (batch_size.get() - 1);
								println!("Received ReadMessage task: {:?}", task_id);
								for item in start_block..task_details.start + 1 {
									let contains_gmp_task = all_gmp_blocks.iter().any(|block| *block == item);
									if contains_gmp_task {
										bench_state.add_recv_task(task_id);
										println!("Contains gmp task");
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
						let task_payload = task_result.1;

						if let Payload::Gmp(ref msgs) = task_payload.payload {
							bench_state.update_recv_gmp_task(task_id, msgs.len() as u64);
						}

						if bench_state.task_ids().contains(&task_id) || bench_state.recv_task_ids().contains(&task_id) {
							if let Payload::Error(error) = task_payload.payload {
								bench_state.add_errored_tasks(task_id, error);
							}
							bench_state.finish_task(task_id);
							println!("finishing task: {:?}", task_id);
						}
					}
					// update task phase
					bench_state.sync_phase(dest_tester).await;
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
					is_contract_updated = true;
					println!("contract updated, waiting for task to complete: {}", format_duration(bench_state.current_duration()));
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

				// verify if the number of tasks finished matches the number of calls or greater and all tasks are finished
				if bench_state.get_finished_tasks().len() >= number_of_calls as usize
				&& bench_state.all_tasks_completed() {
					println!("all tasks Completed");
					if !is_contract_updated {
						println!("Contract was not able to update completely");
					}
					break;
				} else {
					println!("task_ids: {:?}:{:?}, completed: {:?}:{:?}", bench_state.task_ids().len(), bench_state.task_ids(), bench_state.get_finished_tasks().len(), bench_state.get_finished_tasks());
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

///
/// Runs gmp test flow
///
/// # Arguments:
/// `src`: src chain connected tester
/// `dest`: dest chain connected tester
/// `contract`: test contract path
/// `deposit_amount`: optional deposit amount on gateway contracts, gmp works wihtout it but chronicles are not refunded.
async fn gmp_test(
	src: &Tester,
	dest: &Tester,
	contract: &Path,
	deposit_amount: Option<(u128, u128)>,
) -> Result<()> {
	let (src_contract, dest_contract) = setup_gmp_with_contracts(src, dest, contract).await?;

	let src_proxy = src.geteway().await?.context("Gateway not found")?;
	let dest_proxy = dest.geteway().await?.context("Gateway not found")?;
	let voting_call = VotingContract::voteCall { _vote: true }.abi_encode();
	let msg_size = U256::from_str_radix(&voting_call.len().to_string(), 16).unwrap();

	if let Some((src_deposit, dest_deposit)) = deposit_amount {
		src.deposit_gateway_funds(src_proxy, src_deposit).await?;
		dest.deposit_gateway_funds(dest_proxy, dest_deposit).await?;
	}

	println!("sending estimateMsg cost to: {:?}", src_contract);
	// estimate message_cost
	let result = src
		.wallet()
		.eth_send_call(
			src_proxy,
			Gateway::estimateMessageCostCall {
				networkid: dest.network_id(),
				messageSize: msg_size,
				gasLimit: U256::from(100_000),
			}
			.abi_encode(),
			0,
			None,
			None,
		)
		.await?;

	let SubmitResult::Executed { result, .. } = result else { anyhow::bail!("{:?}", result) };
	let CallResult::Success(data) = result else { anyhow::bail!("failed parsing {:?}", result) };
	let msg_cost: u128 = Gateway::estimateMessageCostCall::abi_decode_returns(&data, true)?
		._0
		.try_into()
		.unwrap();

	// submit a vote on source contract (testing contract) which will emit a gmpcreated event on gateway contract
	let res = src
		.wallet()
		.eth_send_call(src_contract, voting_call, msg_cost, None, None)
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

async fn gmp_funds_check(
	src: &Tester,
	dest: &Tester,
	contract: &Path,
	chronicles: &[Tester],
	timechain_url: &str,
) -> Result<()> {
	let (src_contract, _) = setup_gmp_with_contracts(src, dest, contract).await?;

	let subxt_client = SubxtClient::get_client(timechain_url).await?;

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
	let chronicle_wallet = chronicles.first().unwrap().wallet();
	let current_balance = chronicle_wallet.balance().await?;
	// leave some space for gas_price
	let transfer_balance = current_balance - 100000000000000u128;
	println!("Emptying chronicle balance");
	chronicle_wallet
		.transfer(src.wallet().account(), transfer_balance, None, None)
		.await
		.unwrap();
	println!("looking for unregister event");
	let mut block_stream = src.finality_block_stream().await;

	'main: loop {
		tokio::select! {
			block = block_stream.next() => {
				if let Some((block_hash, block_number)) = block {
					println!("Received block number: {:?}", block_number);
					let events = subxt_client.events().at(block_hash).await?;
					let member_offline_event = events.find::<events::UnRegisteredMember>().flatten().next();

					if let Some(member_offline) = member_offline_event {
						let network = member_offline.1;

						println!("member offline for network: {:?}", network);
						break 'main;
					}
				}
			}
			_ = tokio::signal::ctrl_c() => {
				println!("aborting...");
				anyhow::bail!("abort");
			}
		}
	}
	println!("Test Passed");
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
