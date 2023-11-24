use std::u8;

use crate::mock::*;
use crate::tasks::*;
use shards as Shards;

use clap::Parser;
use num_bigint::BigUint;
use rosetta_client::Blockchain;
use tc_subxt::Network;
use tc_subxt::SubxtClient;

mod mock;
mod shards;
mod tasks;

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_value = "ws://validator:9944")]
	url: String,
	#[arg(long)]
	network: String,
	#[clap(subcommand)]
	cmd: TestCommand,
}

#[derive(Parser, Debug)]
enum TestCommand {
	Basic,
	BasicSign,
	BatchTask { tasks: u64, max_cycle: u64 },
	KeyRecovery { nodes: u8 },
	ShardRestart,
	DeployContract,
	Gmp,
	FundWallet,
	SetKeys,
	WatchTask { task_id: u64 },
}

#[derive(Parser, Debug)]
struct InsertSignTaskParams {
	#[arg(default_value_t = 2)]
	cycle: u64,
	#[arg(default_value_t = 20)]
	start: u64,
	#[arg(default_value_t = 2)]
	period: u64,
	#[arg(default_value_t = String::new())]
	contract_address: String,
	#[arg(default_value_t = String::new())]
	payload: String,
}

#[derive(Parser, Debug)]
struct InsertTaskParams {
	#[arg(default_value_t=String::from(""))]
	address: String,
	#[arg(default_value_t = 2)]
	cycle: u64,
	#[arg(default_value_t = 20)]
	start: u64,
	#[arg(default_value_t = 2)]
	period: u64,
	#[arg(default_value_t = false)]
	is_payable: bool,
}

#[tokio::main]
async fn main() {
	let args = Args::parse();
	let url = args.url;
	let api = loop {
		let Ok(api) = SubxtClient::new(&url, None).await else {
			println!("waiting for chain to start");
			tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
			continue;
		};
		break api;
	};

	let (network, config) = match args.network.as_str() {
		"ethereum" => (
			Network::Ethereum,
			WalletConfig {
				blockchain: Blockchain::Ethereum,
				network: "dev".to_string(),
				url: "ws://ethereum:8545".to_string(),
			},
		),
		"astar" => (
			Network::Astar,
			WalletConfig {
				blockchain: Blockchain::Astar,
				network: "dev".to_string(),
				url: "ws://astar:9944".to_string(),
			},
		),
		network => panic!("unsupported network {}", network),
	};

	match args.cmd {
		TestCommand::Basic => {
			basic_test_timechain(&api, network, &config).await;
		},
		TestCommand::BasicSign => {
			basic_sign_test(&api, network, &config).await;
		},
		TestCommand::BatchTask { tasks, max_cycle } => {
			batch_test(&api, tasks, max_cycle, &config).await;
		},
		TestCommand::Gmp => process_gmp_task(&api, &config).await,
		TestCommand::KeyRecovery { nodes } => {
			key_recovery_after_drop(&api, &config, nodes).await;
		},
		TestCommand::ShardRestart => {
			task_update_after_shard_offline(&api, &config).await;
		},
		TestCommand::SetKeys => {
			set_keys(&config).await;
		},
		TestCommand::FundWallet => {
			fund_wallet(&config).await;
		},
		TestCommand::DeployContract => {
			if let Err(e) = deploy_contract(&config, false).await {
				println!("error {:?}", e);
			}
		},
		TestCommand::WatchTask { task_id } => {
			while !watch_task(&api, task_id).await {
				tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
			}
		},
	}
}

async fn process_gmp_task(api: &SubxtClient, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, true).await;

	while !Shards::is_shard_online(api, Network::Ethereum).await {
		println!("Waiting for shard to go online");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	// get shard commitment
	let public_key = api.shard_public_key(0).await.unwrap();
	println!("shard key {:?}", public_key);
	// register commitment with contract
	let revoke: Vec<u8> = vec![];
	let revoke_coord: Vec<Vec<u8>> = vec![];

	let parity = vec![public_key[0]];
	let mut x_coord = [0u8; 32];
	x_coord.copy_from_slice(&public_key[1..]);
	let x_coord_uint = BigUint::from_bytes_be(&x_coord);
	let x_coord = vec![x_coord_uint];
	println!("{:?}-{:?}", parity, x_coord);
	let function = create_gmp_register_call(
		contract_address.clone(),
		vec![
			format!("{:?}", revoke),
			format!("{:?}", revoke_coord),
			format!("{:?}", parity),
			format!("{:?}", x_coord),
		],
	);
	let task_id = insert_task(api, 1, start_block, 0, Network::Ethereum, function).await.unwrap();
	println!("task_id {:?}", task_id);
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
	// then execute gmp task
	//TODO fix which task to call
	// let task_id = insert_sign_task(
	// 	api,
	// 	1, //cycle
	// 	0, //period
	// 	start_block,
	// 	Network::Ethereum,
	// 	contract_address.into(),
	// 	"vote_yes()".into(), //payload
	// )
	// .await
	// .unwrap();
	// while !watch_task(api, task_id).await {
	// 	tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	// }
}

async fn basic_test_timechain(api: &SubxtClient, network: Network, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, false).await;

	let call = create_evm_view_call(contract_address.clone());
	let task_id = insert_task(
		api,
		2, //cycle
		start_block,
		2, //period
		network.clone(),
		call,
	)
	.await
	.unwrap();
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	let paid_call = create_evm_call(contract_address);
	let task_id = insert_task(api, 1, start_block, 0, network, paid_call).await.unwrap();
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn basic_sign_test(api: &SubxtClient, network: Network, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, true).await;

	let call = create_sign_task(contract_address.into(), "vote_yes()".into());
	let task_id = insert_task(
		api,
		1, //cycle
		0, //period
		start_block,
		network.clone(),
		call,
	)
	.await
	.unwrap();
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn batch_test(api: &SubxtClient, total_tasks: u64, max_cycle: u64, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, false).await;

	let mut task_ids = vec![];
	let call = create_evm_view_call(contract_address);
	for _ in 0..total_tasks {
		let task_id = insert_task(
			api,
			max_cycle,
			start_block,
			2, //period
			Network::Ethereum,
			call.clone(),
		)
		.await
		.unwrap();
		task_ids.push(task_id);
	}
	while !watch_batch(api, task_ids[0], task_ids.len() as u64, max_cycle).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn task_update_after_shard_offline(api: &SubxtClient, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, false).await;

	let call = create_evm_view_call(contract_address);
	let task_id = insert_task(
		api,
		10, //cycle
		start_block,
		5, //period
		Network::Ethereum,
		call,
	)
	.await
	.unwrap();
	// wait for some cycles to run, Note: tasks are running in background
	tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

	// drop 2 nodes
	drop_node("testnet-chronicle-eth1-1".to_string());
	drop_node("testnet-chronicle-eth1-1".to_string());
	println!("dropped 2 nodes");

	// wait for some time
	while Shards::is_shard_online(api, Network::Ethereum).await {
		println!("Waiting for shard offline");
		tokio::time::sleep(tokio::time::Duration::from_secs(50)).await;
	}
	println!("Shard is offline, starting nodes");

	// start nodes again
	start_node("validator1".to_string());
	start_node("validator2".to_string());

	// watch task
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn key_recovery_after_drop(api: &SubxtClient, config: &WalletConfig, nodes_to_restart: u8) {
	let (contract_address, start_block) = setup_env(config, false).await;

	let call = create_evm_view_call(contract_address);
	let task_id = insert_task(
		api,
		10, //cycle
		start_block,
		2, //period
		Network::Ethereum,
		call,
	)
	.await
	.unwrap();
	// wait for some cycles to run, Note: tasks are running in background
	for i in 1..nodes_to_restart + 1 {
		println!("waiting for 1 min");
		tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
		println!("restarting node {}", i);
		restart_node(format!("testnet-chronicle-eth-{}", i));
	}
	println!("waiting for 20 secs to let node recover completely");
	tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
	// watch task
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}
