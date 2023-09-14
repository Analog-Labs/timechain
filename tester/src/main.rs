use crate::tasks as Tasks;

use anyhow::{Context, Result};
use clap::Parser;
use ethers_solc::{artifacts::Source, CompilerInput, Solc};
use polkadot::runtime_types::time_primitives::shard::Network;
use rosetta_client::{Blockchain, Wallet};
use std::collections::BTreeMap;
use std::path::Path;
use subxt::rpc::{rpc_params, RpcParams};
use subxt::{OnlineClient, PolkadotConfig};

mod tasks;

#[subxt::subxt(runtime_metadata_path = "../infra/metadata.scale")]
pub mod polkadot {}

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_value = "ws://127.0.0.1:9943")]
	url: String,
	#[clap(subcommand)]
	cmd: TestCommand,
}

#[derive(Parser, Debug)]
enum TestCommand {
	Basic,
	BatchTaskEth { tasks: u64, max_cycle: u64 },
	BatchTaskAstar { tasks: u64, max_cycle: u64 },
	DeployEthContract,
	DeployAstarContract,
	FundEthWallets,
	FundAstarWallets,
	InsertTask(InsertTaskParams),
	SetKeys,
	WatchTask { task_id: u64 },
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
	#[arg(default_value_t=String::from("ethereum"))]
	network: String,
	#[arg(default_value_t = false)]
	is_payable: bool,
}

#[derive(Clone, Debug)]
struct WalletConfig {
	pub blockchain: Blockchain,
	pub network: String,
	pub url: String,
}

#[tokio::main]
async fn main() {
	let args = Args::parse();
	let url = args.url;
	let api = OnlineClient::<PolkadotConfig>::from_url(url).await.unwrap();

	let eth_config = WalletConfig {
		blockchain: Blockchain::Ethereum,
		network: "dev".to_string(),
		url: "ws://127.0.0.1:8545".to_string(),
	};

	let astar_config = WalletConfig {
		blockchain: Blockchain::Astar,
		network: "dev".to_string(),
		url: "ws://127.0.0.1:9944".to_string(),
	};

	match args.cmd {
		TestCommand::Basic => {
			basic_test_timechain(&api, eth_config, astar_config).await;
		},
		TestCommand::BatchTaskEth { tasks, max_cycle } => {
			batch_test(&api, tasks, max_cycle, eth_config).await;
		},
		TestCommand::BatchTaskAstar { tasks, max_cycle } => {
			batch_test(&api, tasks, max_cycle, astar_config).await;
		},
		TestCommand::SetKeys => {
			set_keys().await;
		},
		TestCommand::FundEthWallets => {
			fund_wallets(eth_config).await;
		},
		TestCommand::FundAstarWallets => {
			fund_wallets(astar_config).await;
		},
		TestCommand::DeployEthContract => {
			if let Err(e) = deploy_eth_contract(eth_config).await {
				println!("error {:?}", e);
			}
		},
		TestCommand::DeployAstarContract => {
			if let Err(e) = deploy_astar_contract(astar_config).await {
				println!("error {:?}", e);
			}
		},
		TestCommand::InsertTask(params) => {
			let task_id = Tasks::insert_evm_task(
				&api,
				params.address,
				params.cycle,
				params.start,
				params.period,
				if params.network == "ethereum" { Network::Ethereum } else { Network::Astar },
				params.is_payable,
			)
			.await
			.unwrap();
			println!("Registered task_id {:?}", task_id);
		},
		TestCommand::WatchTask { task_id } => {
			while let false = Tasks::watch_task(&api, task_id).await {
				tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
			}
		},
	}
}

async fn basic_test_timechain(
	api: &OnlineClient<PolkadotConfig>,
	eth_config: WalletConfig,
	astar_config: WalletConfig,
) {
	set_keys().await;
	fund_wallets(eth_config.clone()).await;
	let (contract_address, start_block) = deploy_eth_contract(eth_config).await.unwrap();

	//eth viewcall task
	let task_id = Tasks::insert_evm_task(
		api,
		contract_address.clone(),
		2,
		start_block,
		2,
		Network::Ethereum,
		false,
	)
	.await
	.unwrap();
	while let false = Tasks::watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	//eth payable task
	let task_id =
		Tasks::insert_evm_task(api, contract_address, 1, start_block, 0, Network::Ethereum, true)
			.await
			.unwrap();
	while let false = Tasks::watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}

	//astar testing
	fund_wallets(astar_config.clone()).await;
	let (contract_address, start_block) = deploy_astar_contract(astar_config).await.unwrap();
	let task_id =
		Tasks::insert_evm_task(api, contract_address, 2, start_block, 2, Network::Astar, false)
			.await
			.unwrap();
	while let false = Tasks::watch_task(api, task_id).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn batch_test(
	api: &OnlineClient<PolkadotConfig>,
	total_tasks: u64,
	max_cycle: u64,
	config: WalletConfig,
) {
	set_keys().await;
	fund_wallets(config.clone()).await;
	let mut task_ids = vec![];
	let (contract_address, start_block) = if config.blockchain == Blockchain::Ethereum {
		deploy_eth_contract(config).await.unwrap()
	} else {
		deploy_astar_contract(config).await.unwrap()
	};

	for _ in 0..total_tasks {
		let task_id = Tasks::insert_evm_task(
			api,
			contract_address.clone(),
			max_cycle,
			start_block,
			2,
			Network::Ethereum,
			false,
		)
		.await
		.unwrap();
		task_ids.push(task_id);
	}
	while let false = Tasks::watch_batch(api, task_ids[0], task_ids.len() as u64, max_cycle).await {
		tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
	}
}

async fn set_keys() {
	let mut success_sets = 0;
	let start_port = 9943;
	let keys = [
		"0x78af33d076b81fddce1c051a72bb1a23fd32519a2ede7ba7a54b2c76d110c54d",
		"0xcee262950a61e921ac72217fd5578c122bfc91ba5c0580dbfbe42148cf35be2b",
		"0xa01b6ceec7fb1d32bace8ffcac21ffe6839d3a2ebe26d86923be9dd94c0c9a02",
		"0x1e31bbe09138bef48ffaca76214317eb0f7a8fd85959774e41d180f2ad9e741f",
		"0x1843caba7078a699217b23bcec8b57db996fc3d1804948e9ee159fc1dc9b8659",
		"0x72a170526bb41438d918a9827834c38aff8571bfe9203e38b7a6fd93ecf70d69",
	];
	while success_sets < 6 {
		success_sets = 0;
		for i in 0..6 {
			let suri = format!("owner word vocal dose decline sunset battle example forget excite gentle waste//{}//time", i+1);
			let node_port = start_port + (i * 2);
			let url = format!("ws://127.0.0.1:{}", node_port);
			let Ok(api) = OnlineClient::<PolkadotConfig>::from_url(url).await else{
				println!("failed to connect to node {}", i+1);
				continue;
			};
			let params: RpcParams = rpc_params!["time", suri, keys[i]];
			let _: () = api.rpc().request("author_insertKey", params).await.unwrap();
			success_sets += 1;
			println!("submitted key for node: {}", i + 1);
		}
		if success_sets < 6 {
			println!("===========retrying set keys==========");
			tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
		}
	}
}

async fn fund_wallets(config: WalletConfig) {
	println!("funding walllets");
	let read_files = std::fs::read_dir("./dummy_wallets").unwrap();
	let is_eth = config.blockchain == Blockchain::Ethereum;
	for file in read_files {
		let file = file.unwrap().path();
		let cloned_config = config.clone();
		let wallet = Wallet::new(
			cloned_config.blockchain,
			&cloned_config.network,
			&cloned_config.url,
			Some(&file),
		)
		.await
		.unwrap();
		let file_string = file.to_str().unwrap();
		if is_eth && file_string.contains("eth") {
			// keyfile 1 needed to be funded for contract deployment
			if file_string.contains("1") {
				let _ = wallet.faucet(10000000000000000).await;
			} else {
				tokio::spawn(async move {
					let _ = wallet.faucet(10000000000000000).await;
				});
			}
		} else if !is_eth && file_string.contains("astar") {
			// if run in above way throws low priority error must await
			wallet.faucet(10000000000000000000).await.unwrap();
		}
	}
}

async fn deploy_eth_contract(eth_config: WalletConfig) -> Result<(String, u64)> {
	println!("Deploying eth contract");
	let wallet = Wallet::new(
		eth_config.blockchain,
		&eth_config.network,
		&eth_config.url,
		Some(Path::new("./dummy_wallets/eth_keyfile1")),
	)
	.await
	.unwrap();

	let bytes = compile_file("./contracts/test_contract.sol")?;
	let tx_hash = wallet.eth_deploy_contract(bytes).await?;
	let tx_receipt = wallet.eth_transaction_receipt(&tx_hash).await?;
	let contract_address = tx_receipt
		.get("contractAddress")
		.and_then(|v| v.as_str().map(str::to_string))
		.ok_or(anyhow::anyhow!("Unable to get contract address"))?;
	let status = wallet.status().await?;

	println!("Deploy contract address {:?} on {:?}", contract_address, status.index);
	Ok((contract_address.to_string(), status.index))
}

async fn deploy_astar_contract(astar_config: WalletConfig) -> Result<(String, u64)> {
	println!("Deploying astar contract");
	let wallet = Wallet::new(
		astar_config.blockchain,
		&astar_config.network,
		&astar_config.url,
		Some(Path::new("./dummy_wallets/astar_keyfile1")),
	)
	.await
	.unwrap();

	let bytes = compile_file("./contracts/test_contract.sol")?;
	let tx_hash = wallet.eth_deploy_contract(bytes).await?;
	let tx_receipt = wallet.eth_transaction_receipt(&tx_hash).await?;
	let contract_address = tx_receipt
		.get("contractAddress")
		.and_then(|v| v.as_str().map(str::to_string))
		.ok_or(anyhow::anyhow!("Unable to get contract address"))?;
	let status = wallet.status().await?;

	println!("Deploy contract address {:?} on {:?}", contract_address, status.index);
	Ok((contract_address.to_string(), status.index + 5))
}

fn compile_file(path: &str) -> Result<Vec<u8>> {
	let solc = Solc::default();
	let mut sources = BTreeMap::new();
	sources.insert(Path::new(path).into(), Source::read(path).unwrap());
	let input = &CompilerInput::with_sources(sources)[0];
	let output = solc.compile_exact(input)?;
	let file = output.contracts.get(path).unwrap();
	let (key, _) = file.first_key_value().unwrap();
	let contract = file.get(key).unwrap();
	let bytecode = contract
		.evm
		.as_ref()
		.context("evm not found")?
		.bytecode
		.as_ref()
		.context("bytecode not found")?
		.object
		.as_bytes()
		.context("could not convert to bytes")?
		.to_vec();
	Ok(bytecode)
}
