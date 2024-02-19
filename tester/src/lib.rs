use alloy_primitives::U256;
use alloy_sol_types::{sol, SolValue};
use anyhow::{Context, Result};
use ethers_solc::{artifacts::Source, CompilerInput, Solc};
use rosetta_client::Wallet;
use sha3::{Digest, Keccak256};
use sp_core::crypto::Ss58Codec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tc_subxt::timechain_runtime::runtime_types::time_primitives::task::TaskStatus;
use tc_subxt::timechain_runtime::tasks::events::{GatewayRegistered, TaskCreated};
use tc_subxt::{SubxtClient, SubxtTxSubmitter};
use time_primitives::{
	Function, NetworkId, Runtime, ShardId, TaskDescriptorParams, TaskId, TssPublicKey,
};

pub struct TesterParams {
	pub network_id: NetworkId,
	pub timechain_keyfile: PathBuf,
	pub timechain_url: String,
	pub target_keyfile: PathBuf,
	pub target_url: String,
	pub gateway_contract: PathBuf,
}

pub struct Tester {
	network_id: NetworkId,
	gateway_contract: PathBuf,
	runtime: SubxtClient,
	wallet: Wallet,
}

impl Tester {
	pub async fn new(args: TesterParams) -> Result<Self> {
		while SubxtClient::get_client(&args.timechain_url).await.is_err() {
			println!("waiting for chain to start");
			tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
		}

		let tx_submitter = SubxtTxSubmitter::try_new(&args.timechain_url).await.unwrap();
		let runtime =
			SubxtClient::with_keyfile(&args.timechain_url, &args.timechain_keyfile, tx_submitter)
				.await?;
		println!("tester key is {:?}", runtime.account_id().to_ss58check());

		let (blockchain, network) = runtime
			.get_network(args.network_id)
			.await?
			.ok_or(anyhow::anyhow!("Unknown network id"))?;

		let wallet = Wallet::new(
			blockchain.parse()?,
			&network,
			&args.target_url,
			Some(&args.target_keyfile),
		)
		.await?;
		Ok(Self {
			network_id: args.network_id,
			gateway_contract: args.gateway_contract,
			runtime,
			wallet,
		})
	}

	pub fn network_id(&self) -> NetworkId {
		self.network_id
	}

	pub async fn faucet(&self) {
		if let Err(err) = self.wallet.faucet(1000000000000000000000).await {
			println!("Error occured while funding wallet {:?}", err);
		}
	}

	pub async fn deploy(&self, path: &Path, constructor: &[u8]) -> Result<(String, u64)> {
		println!("Deploying contract from {:?}", self.wallet.account().address);
		let mut contract = compile_file(path)?;
		contract.extend(constructor);
		let tx_hash = self.wallet.eth_deploy_contract(contract).await?;
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash).await?;
		let contract_address = format!("{:?}", tx_receipt.unwrap().contract_address.unwrap());
		let status = self.wallet.status().await?;

		println!("Deploy contract address {:?} on {:?}", contract_address, status.index);
		Ok((contract_address.to_string(), status.index))
	}

	pub async fn deploy_gateway(&self, tss_public_key: TssPublicKey) -> Result<(String, u64)> {
		let parity_bit = if tss_public_key[0] % 2 == 0 { 0 } else { 1 };
		let x_coords = hex::encode(&tss_public_key[1..]);
		sol! {
			#[derive(Debug, PartialEq, Eq)]
			struct TssKey {
				uint8 yParity;
				uint256 xCoord;
			}
		}
		let tss_keys = vec![TssKey {
			yParity: parity_bit,
			xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
		}];
		let constructor = tss_keys.abi_encode_params();
		self.deploy(&self.gateway_contract, &constructor).await
	}

	pub async fn get_shard_id(&self) -> Result<Option<ShardId>> {
		let shard_id_counter = self.runtime.shard_id_counter().await?;
		for shard_id in 0..shard_id_counter {
			let shard_network = self.runtime.shard_network(shard_id).await?;
			if shard_network == self.network_id {
				return Ok(Some(shard_id));
			}
		}
		Ok(None)
	}

	pub async fn is_shard_online(&self, shard_id: u64) -> bool {
		use tc_subxt::timechain_runtime::runtime_types::time_primitives::shard::ShardStatus;
		self.runtime.shard_state(shard_id).await.unwrap() == ShardStatus::Online
	}

	pub async fn wait_for_shard(&self) -> Result<ShardId> {
		let shard_id = loop {
			let Some(shard_id) = self.get_shard_id().await? else {
				println!("Waiting for shard to be created");
				tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
				continue;
			};
			break shard_id;
		};
		while !self.is_shard_online(shard_id).await {
			println!("Waiting for shard to go online");
			tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
		}
		Ok(shard_id)
	}

	pub async fn create_task(&self, function: Function, block: u64) -> Result<TaskId> {
		let params = TaskDescriptorParams {
			network: self.network_id,
			function,
			start: block,
			shard_size: 1,
			funds: 10000000000000000,
		};
		let events = self.runtime.create_task(params).await?.wait_for_finalized_success().await?;
		let transfer_event = events.find_first::<TaskCreated>().unwrap();
		let TaskCreated(id) =
			transfer_event.ok_or(anyhow::anyhow!("Not able to fetch task event"))?;
		println!("Task registered: {:?}", id);
		Ok(id)
	}

	pub async fn register_gateway_address(&self, shard_id: u64, address: &str) -> Result<()> {
		let address_bytes = get_eth_address_to_bytes(address);
		let events = self
			.runtime
			.insert_gateway(shard_id, address_bytes)
			.await?
			.wait_for_finalized_success()
			.await?;
		let gateway_event = events.find_first::<GatewayRegistered>().unwrap();
		println!("Gateway registered with event {:?}", gateway_event);
		Ok(())
	}

	pub async fn is_task_finished(&self, task_id: TaskId) -> bool {
		let task_state = self.runtime.get_task_state(task_id).await.unwrap();
		matches!(task_state, Some(TaskStatus::Completed) | Some(TaskStatus::Failed { .. }))
	}

	pub async fn wait_for_task(&self, task_id: TaskId) {
		while !self.is_task_finished(task_id).await {
			println!("task id: {:?} still running", task_id);
			tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
		}
	}

	pub async fn create_task_and_wait(&self, function: Function, block: u64) {
		let task_id = self.create_task(function, block).await.unwrap();
		self.wait_for_task(task_id).await
	}

	pub async fn setup_gmp(&self) -> Result<()> {
		let shard_id = self.wait_for_shard().await?;
		let shard_public_key = self.runtime.shard_public_key(shard_id).await.unwrap();
		let (address, _) = self.deploy_gateway(shard_public_key).await?;
		self.register_gateway_address(shard_id, &address).await
	}

	pub async fn get_latest_block(&self) -> Result<u64> {
		self.runtime.get_latest_block().await
	}
}

fn compile_file(path: &Path) -> Result<Vec<u8>> {
	let solc = Solc::default();
	let mut sources = BTreeMap::new();
	sources.insert(path.into(), Source::read(path).unwrap());
	let input = &CompilerInput::with_sources(sources)[0];
	let output = solc.compile_exact(input)?;
	let file = output.contracts.get(path.to_str().unwrap()).unwrap();
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

pub fn drop_node(node_name: String) {
	let output = Command::new("docker")
		.args(["stop", &node_name])
		.output()
		.expect("failed to drop node");
	println!("Dropped node {:?}", output.stderr.is_empty());
}

pub fn start_node(node_name: String) {
	let output = Command::new("docker")
		.args(["start", &node_name])
		.output()
		.expect("failed to start node");
	println!("Start node {:?}", output.stderr.is_empty());
}

pub fn restart_node(node_name: String) {
	let output = Command::new("docker")
		.args(["restart", &node_name])
		.output()
		.expect("failed to start node");
	println!("Restart node {:?}", output.stderr.is_empty());
}

pub fn create_evm_call(address: String) -> Function {
	Function::EvmCall {
		address: get_eth_address_to_bytes(&address),
		input: get_evm_function_hash("vote_yes()"),
		amount: 0,
	}
}

pub fn create_evm_view_call(address: String) -> Function {
	Function::EvmViewCall {
		address: get_eth_address_to_bytes(&address),
		input: get_evm_function_hash("get_votes_stats()"),
	}
}

pub fn create_send_msg_call(
	address: String,
	function: &str,
	salt: [u8; 32],
	gas_limit: u64,
) -> Function {
	Function::SendMessage {
		address: get_eth_address_to_bytes(&address),
		payload: get_evm_function_hash(function),
		salt,
		gas_limit,
	}
}

fn get_evm_function_hash(arg: &str) -> Vec<u8> {
	let mut hasher = Keccak256::new();
	hasher.update(arg);
	let hash = hasher.finalize();
	hash[..4].into()
}

fn get_eth_address_to_bytes(address: &str) -> [u8; 20] {
	let mut eth_bytes = [0u8; 20];
	let trimmed_address = address.trim_start_matches("0x");
	hex::decode_to_slice(trimmed_address, &mut eth_bytes).unwrap();
	eth_bytes
}
