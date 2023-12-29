use crate::mock::*;
use crate::tasks::*;
use shards as Shards;

use clap::Parser;
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
	BatchTask { tasks: u64, max_cycle: u64 },
	KeyRecovery { nodes: u8 },
	ShardRestart,
	DeployGatewayContract { shard_id: u64 },
	Gmp,
	FundWallet,
	WatchTask { task_id: u64 },
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

	let eth_config = WalletConfig {
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
		"ethereum" => (Network::Ethereum, eth_config.clone()),
		"astar" => (Network::Astar, astar_config.clone()),
		network => panic!("unsupported network {}", network),
	};

	match args.cmd {
		TestCommand::Basic => {
			basic_test_timechain(&api, network, &config).await;
		},
		TestCommand::BatchTask { tasks, max_cycle } => {
			batch_test(&api, tasks, max_cycle, &config).await;
		},
		TestCommand::KeyRecovery { nodes } => {
			key_recovery_after_drop(&api, &config, nodes).await;
		},
		TestCommand::ShardRestart => {
			task_update_after_shard_offline(&api, &config).await;
		},
		TestCommand::FundWallet => {
			fund_wallet(&config).await;
		},
		TestCommand::DeployGatewayContract { shard_id } => {
			deploy_gateway_contract(&api, &config, shard_id).await
		},
		TestCommand::WatchTask { task_id } => {
			while !watch_task(&api, task_id).await {
				tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
			}
		},
		TestCommand::Gmp => process_gmp_task(&api, &eth_config, &astar_config).await,
	}
}

async fn deploy_gateway_contract(api: &SubxtClient, config: &WalletConfig, shard_id: u64) {
	while !Shards::is_shard_online(api, shard_id).await {
		println!("Waiting for eth shard to go online");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
	let shard_public_key = api.shard_public_key(shard_id).await.unwrap();
	let constructor_params = get_gmp_constructor_params(shard_public_key);
}

async fn process_gmp_task(
	api: &SubxtClient,
	eth_config: &WalletConfig,
	astar_config: &WalletConfig,
) {
	let eth_shard_id = Shards::get_shard_id(api, Network::Ethereum).await;
	let astar_shard_id = Shards::get_shard_id(api, Network::Astar).await;
	while !Shards::is_shard_online(api, eth_shard_id).await {
		println!("Waiting for eth shard to go online");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
	while !Shards::is_shard_online(api, astar_shard_id).await {
		println!("Waiting for astar shard to go online");
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	let eth_public_key = api.shard_public_key(eth_shard_id).await.unwrap();
	let astar_public_key = api.shard_public_key(astar_shard_id).await.unwrap();
	let eth_constructor_params = get_gmp_constructor_params(eth_public_key);
	let astar_constructor_params = get_gmp_constructor_params(astar_public_key);

	let (eth_contract_address_gmp, eth_start_block_gmp) =
		setup_env(eth_config, Some(eth_constructor_params)).await;
	let (eth_contract_address, _eth_start_block) = setup_env(eth_config, None).await;
	println!("Setup for eth done");
	let (astar_contract_address_gmp, _astar_start_block_gmp) =
		setup_env(astar_config, Some(astar_constructor_params)).await;
	println!("Setup for astar done");

	//register_gateway contract
	register_gateway_address(api, eth_shard_id, &eth_contract_address_gmp)
		.await
		.unwrap();
	register_gateway_address(api, astar_shard_id, &astar_contract_address_gmp)
		.await
		.unwrap();

	let send_msg = create_send_msg_call(eth_contract_address, "vote_yes()", [1; 32], 10000000);
	let task_id = insert_task(
		api,
		1, //cycle
		eth_start_block_gmp,
		0, //period
		Network::Ethereum,
		send_msg,
	)
	.await
	.unwrap();
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn basic_test_timechain(api: &SubxtClient, network: Network, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, None).await;

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

async fn batch_test(api: &SubxtClient, total_tasks: u64, max_cycle: u64, config: &WalletConfig) {
	let (contract_address, start_block) = setup_env(config, None).await;

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
	let (contract_address, start_block) = setup_env(config, None).await;

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
	let shard_id = Shards::get_shard_id(api, Network::Ethereum).await;
	while Shards::is_shard_online(api, shard_id).await {
		println!("Waiting for shard offline");
		tokio::time::sleep(tokio::time::Duration::from_secs(50)).await;
	}
	println!("Shard is offline, starting nodes");

	// start nodes again
	start_node("testnet-chronicle-eth1-1".to_string());
	start_node("testnet-chronicle-eth1-1".to_string());

	// watch task
	while !watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn key_recovery_after_drop(api: &SubxtClient, config: &WalletConfig, nodes_to_restart: u8) {
	let (contract_address, start_block) = setup_env(config, None).await;

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
