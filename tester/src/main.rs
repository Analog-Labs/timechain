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
	InsertTask(InsertTaskParams),
	InsertSignTask(InsertSignTaskParams),
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

	let ethereum_config = WalletConfig {
		blockchain: Blockchain::Ethereum,
		network: "dev".to_string(),
		url: "ws://ethereum:8545".to_string(),
	};

	let astar_config = WalletConfig {
		blockchain: Blockchain::Astar,
		network: "dev".to_string(),
		url: "ws://astar:9944".to_string(),
	};

	let (network, config) = match args.network.as_str() {
		"ethereum" => (Network::Ethereum, ethereum_config.clone()),
		"astar" => (Network::Astar, astar_config.clone()),
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
		TestCommand::Gmp => process_gmp_task(&api, &ethereum_config, &astar_config).await,
		TestCommand::KeyRecovery { nodes } => {
			key_recovery_after_drop(&api, &config, nodes).await;
		},
		TestCommand::ShardRestart => {
			task_update_after_shard_offline(&api, &config).await;
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

// For GMP Task you must start both ethereum and astar nodes
async fn process_gmp_task(
	api: &SubxtClient,
	eth_config: &WalletConfig,
	astar_config: &WalletConfig,
) {
	let (eth_contract_address, eth_start_block) = setup_env(eth_config, true).await;
	println!("Setup for eth done");
	let (astar_contract_address, astar_start_block) = setup_env(astar_config, true).await;
	println!("Setup for astar done");

	let eth_shard_id = Shards::get_shard_id(api, Network::Ethereum).await;
	let astar_shard_id = Shards::get_shard_id(api, Network::Astar).await;
	while !Shards::is_shard_online(api, eth_shard_id).await {
		println!("Waiting for shard to go online");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	while !Shards::is_shard_online(api, astar_shard_id).await {
		println!("Waiting for shard to go online");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	register_eth_gmp(api, eth_shard_id, eth_contract_address.clone(), eth_start_block).await;
	register_astar_gmp(api, astar_shard_id, astar_contract_address, astar_start_block).await;

	let function = create_sign_task(eth_contract_address.into(), "vote_yes".into());
	let task_id = insert_task(
		api,
		1, //cycle
		0, //period
		astar_start_block,
		Network::Astar,
		function,
	)
	.await
	.unwrap();
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn register_astar_gmp(
	api: &SubxtClient,
	astar_shard_id: u64,
	astar_contract_address: String,
	astar_start_block: u64,
) {
	//for now we just register astar public key to do gmp tasks, so calls can be processed on ethereum shard.
	let astar_public_key = api.shard_public_key(astar_shard_id).await.unwrap();
	println!("astar shard key {:?}", astar_public_key);

	let revoke: Vec<u8> = vec![];
	let revoke_coord: Vec<Vec<u8>> = vec![];

	let parity = vec![astar_public_key[0]];
	let mut x_coord = [0u8; 32];
	x_coord.copy_from_slice(&astar_public_key[1..]);
	let x_coord_uint = BigUint::from_bytes_be(&x_coord);
	let x_coord = vec![x_coord_uint];
	let function = create_gmp_register_call(
		astar_contract_address.clone(),
		vec![
			format!("{:?}", revoke),
			format!("{:?}", revoke_coord),
			format!("{:?}", parity),
			format!("{:?}", x_coord),
		],
	);
	let task_id = insert_task(api, 1, astar_start_block, 0, Network::Astar, function)
		.await
		.unwrap();
	println!("task_id {:?}", task_id);
	while !watch_task(api, task_id).await {
		println!("waiting for astar gmp register task to complete");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn register_eth_gmp(
	api: &SubxtClient,
	eth_shard_id: u64,
	eth_contract_address: String,
	eth_start_block: u64,
) {
	//for now we just register eth public key to do gmp tasks, so calls can be processed on ethereum shard.
	let eth_public_key = api.shard_public_key(eth_shard_id).await.unwrap();
	println!("eth shard key {:?}", eth_public_key);

	let revoke: Vec<u8> = vec![];
	let revoke_coord: Vec<Vec<u8>> = vec![];

	let parity = vec![eth_public_key[0]];
	let mut x_coord = [0u8; 32];
	x_coord.copy_from_slice(&eth_public_key[1..]);
	let x_coord_uint = BigUint::from_bytes_be(&x_coord);
	let x_coord = vec![x_coord_uint];
	let function = create_gmp_register_call(
		eth_contract_address.clone(),
		vec![
			format!("{:?}", revoke),
			format!("{:?}", revoke_coord),
			format!("{:?}", parity),
			format!("{:?}", x_coord),
		],
	);
	let task_id = insert_task(api, 1, eth_start_block, 0, Network::Ethereum, function)
		.await
		.unwrap();
	println!("task_id {:?}", task_id);
	while !watch_task(api, task_id).await {
		println!("waiting for eth gmp register task to complete");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
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
	let eth_shard_id = Shards::get_shard_id(api, Network::Ethereum).await;
	while Shards::is_shard_online(api, eth_shard_id).await {
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
		restart_node(format!("testnet-chronicle-eth{}-1", i));
	}
	println!("waiting for 20 secs to let node recover completely");
	tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
	// watch task
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}
