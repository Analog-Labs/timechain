use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall, SolConstructor};
use anyhow::Result;
use rosetta_client::Wallet;
use sp_core::crypto::Ss58Codec;
use std::intrinsics::transmute;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tc_subxt::timechain_runtime::tasks::events::{GatewayRegistered, TaskCreated};
use tc_subxt::{SubxtClient, SubxtTxSubmitter};
use time_primitives::sp_core::H160;
use time_primitives::{
	sp_core, Function, IGateway, Msg, NetworkId, Runtime, ShardId, TaskDescriptorParams, TaskId,
	TaskPhase, TssKey, TssPublicKey,
};

pub async fn sleep_or_abort(duration: Duration) -> Result<()> {
	tokio::select! {
		_ = tokio::signal::ctrl_c() => {
			println!("aborting...");
			anyhow::bail!("abort");
		},
		_ = tokio::time::sleep(duration) => Ok(()),
	}
}

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

type TssKeyR = <TssKey as alloy_sol_types::SolType>::RustType;

impl Tester {
	pub async fn new(args: TesterParams) -> Result<Self> {
		while SubxtClient::get_client(&args.timechain_url).await.is_err() {
			println!("waiting for chain to start");
			sleep_or_abort(Duration::from_secs(10)).await?;
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
			None,
		)
		.await?;
		Ok(Self {
			network_id: args.network_id,
			gateway_contract: args.gateway_contract,
			runtime,
			wallet,
		})
	}

	pub fn wallet(&self) -> &Wallet {
		&self.wallet
	}

	pub fn network_id(&self) -> NetworkId {
		self.network_id
	}

	pub async fn faucet(&self) {
		if let Err(err) = self.wallet.faucet(100000000000000000000000000000).await {
			println!("Error occured while funding wallet {:?}", err);
		}
	}

	pub async fn deploy(
		&self,
		path: &Path,
		constructor: impl SolConstructor,
	) -> Result<([u8; 20], u64)> {
		println!("Deploying contract from {:?}", self.wallet.account().address);
		let mut contract = compile_file(path)?;
		contract.extend(constructor.abi_encode());
		let tx_hash = self.wallet.eth_deploy_contract(contract).await?.tx_hash().0;
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash).await?.unwrap();
		let contract_address = tx_receipt.contract_address.unwrap();
		let block_number = tx_receipt.block_number.unwrap();

		println!("Deploy contract address {:?} on {:?}", contract_address, block_number);
		Ok((contract_address.0, block_number))
	}

	pub async fn deploy_gateway(&self, tss_public_key: TssPublicKey) -> Result<([u8; 20], u64)> {
		let parity_bit = if tss_public_key[0] % 2 == 0 { 0 } else { 1 };
		let x_coords = hex::encode(&tss_public_key[1..]);
		sol! {
			#[derive(Debug, PartialEq, Eq)]
			struct TssKey {
				uint8 yParity;
				uint256 xCoord;
			}
		}
		let tss_keys = vec![TssKeyR {
			yParity: parity_bit,
			xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
		}];
		let call = IGateway::constructorCall {
			networkId: self.network_id,
			keys: tss_keys,
		};
		self.deploy(&self.gateway_contract, call).await
	}

	pub async fn deposit_funds(
		&self,
		gmp_address: [u8; 20],
		source_network: NetworkId,
		source: [u8; 20],
		is_contract: bool,
		amount: u128,
	) -> Result<()> {
		println!("depositing funds on destination chain");
		let mut src = [0; 32];
		src[12..32].copy_from_slice(&source[..]);

		// Enable the contract flag
		if is_contract {
			src[11] = 1;
		}
		let payload = IGateway::depositCall {
			network: source_network,
			source: src.into(),
		}
		.abi_encode();
		self.wallet.eth_send_call(gmp_address, payload, amount, None, None).await?;
		Ok(())
	}

	pub async fn get_shard_id(&self) -> Result<Option<ShardId>> {
		let shard_id_counter = self.runtime.shard_id_counter().await?;
		for shard_id in 0..shard_id_counter {
			match self.runtime.shard_network(shard_id).await {
				Ok(shard_network) if shard_network == self.network_id => return Ok(Some(shard_id)),
				Ok(_) => continue,
				Err(err) => {
					println!("Skipping shard_id {shard_id}: {err}");
					continue;
				},
			};
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
				sleep_or_abort(Duration::from_secs(10)).await?;
				continue;
			};
			break shard_id;
		};
		while !self.is_shard_online(shard_id).await {
			println!("Waiting for shard to go online");
			sleep_or_abort(Duration::from_secs(10)).await?;
		}
		Ok(shard_id)
	}

	pub async fn create_task(&self, function: Function, block: u64) -> Result<TaskId> {
		println!("creating task");
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

	pub async fn register_gateway_address(
		&self,
		shard_id: u64,
		address: [u8; 20],
		block_height: u64,
	) -> Result<()> {
		let events = self
			.runtime
			.insert_gateway(shard_id, address, block_height)
			.await?
			.wait_for_finalized_success()
			.await?;
		let gateway_event = events.find_first::<GatewayRegistered>().unwrap();
		println!("Gateway registered with event {:?}", gateway_event);
		Ok(())
	}

	pub async fn is_task_finished(&self, task_id: TaskId) -> bool {
		self.runtime.is_task_complete(task_id).await.unwrap()
	}

	pub async fn get_task_phase(&self, task_id: TaskId) -> TaskPhase {
		let val = self.runtime.get_task_phase(task_id).await.unwrap().expect("Phase not found");
		unsafe { transmute(val) }
	}

	pub async fn wait_for_task(&self, task_id: TaskId) {
		while !self.is_task_finished(task_id).await {
			let phase = self.get_task_phase(task_id).await;
			println!("task id: {} phase {:?} still running", task_id, phase);
			if sleep_or_abort(Duration::from_secs(10)).await.is_err() {
				break;
			}
		}
	}

	pub async fn create_task_and_wait(&self, function: Function, block: u64) {
		let task_id = self.create_task(function, block).await.unwrap();
		self.wait_for_task(task_id).await
	}

	pub async fn setup_gmp(&self, redeploy: bool) -> Result<[u8; 20]> {
		if !redeploy {
			if let Some(gateway) = self.runtime.get_gateway(self.network_id).await? {
				println!("Gateway contract already deployed at {:?}. If you want to redeploy, please use the --redeploy flag.", H160::from_slice(&gateway[..]));
				return Ok(gateway);
			}
		}
		let shard_id = self.wait_for_shard().await?;
		let shard_public_key = self.runtime.shard_public_key(shard_id).await.unwrap();
		let (address, block_height) = self.deploy_gateway(shard_public_key).await?;
		self.register_gateway_address(shard_id, address, block_height).await?;
		Ok(address)
	}

	pub async fn get_latest_block(&self) -> Result<u64> {
		self.runtime.get_latest_block().await
	}

	pub async fn send_message(
		&self,
		source_network: NetworkId,
		source: [u8; 20],
		dest: [u8; 20],
		payload: Vec<u8>,
		gas_limit: u128,
	) -> Result<TaskId> {
		let mut src = [0; 32];
		src[12..32].copy_from_slice(&source[..]);
		let mut salt = [0; 32];
		getrandom::getrandom(&mut salt).unwrap();
		let f = Function::SendMessage {
			msg: Msg {
				source_network,
				source: src,
				dest_network: self.network_id(),
				dest,
				data: payload,
				salt,
				gas_limit,
			},
		};
		self.create_task(f, 0).await
	}
}

fn compile_file(path: &Path) -> Result<Vec<u8>> {
	let abi = std::fs::read_to_string(path)?;
	let json_abi: serde_json::Value = serde_json::from_str(&abi)?;
	Ok(hex::decode(json_abi["bytecode"]["object"].as_str().unwrap().replace("0x", ""))?)
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

sol! {
	#[derive(Debug, PartialEq, Eq)]
	struct GmpVotingContract {
		address dest;
		uint16 network;
	}

	contract VotingContract {
		// Minium gas required for execute the `onGmpReceived` method
		function GMP_GAS_LIMIT() external pure returns(uint256);

		constructor(address _gateway);
		function registerGmpContracts(GmpVotingContract[] memory _registered) external;
		function vote(bool _vote) external;
		function stats() public view returns (uint256[] memory);
	}
}

pub fn create_evm_call(address: [u8; 20]) -> Function {
	Function::EvmCall {
		address,
		input: VotingContract::voteCall { _vote: true }.abi_encode(),
		amount: 0,
		gas_limit: None,
	}
}

pub fn create_evm_view_call(address: [u8; 20]) -> Function {
	Function::EvmViewCall {
		address,
		input: VotingContract::statsCall {}.abi_encode(),
	}
}

pub struct TaskPhaseInfo {
	pub start_block: u64,
	pub write_phase_start: Option<u64>,
	pub read_phase_start: Option<u64>,
	pub finish_block: Option<u64>,
}

impl TaskPhaseInfo {
	pub fn new(start_block: u64) -> Self {
		Self {
			start_block,
			write_phase_start: None,
			read_phase_start: None,
			finish_block: None,
		}
	}

	pub fn enter_write_phase(&mut self, current_block: u64) {
		self.write_phase_start = Some(current_block);
	}

	pub fn enter_read_phase(&mut self, current_block: u64) {
		self.read_phase_start = Some(current_block);
	}

	pub fn task_finished(&mut self, current_block: u64) {
		self.finish_block = Some(current_block);
	}

	pub fn start_to_write_duration(&self) -> Option<u64> {
		self.write_phase_start.map(|write_start| write_start - self.start_block)
	}

	pub fn write_to_read_duration(&self) -> Option<u64> {
		match (self.write_phase_start, self.read_phase_start) {
			(Some(write_start), Some(read_start)) => Some(read_start - write_start),
			_ => None,
		}
	}

	pub fn read_to_finish_duration(&self) -> Option<u64> {
		match (self.read_phase_start, self.finish_block) {
			(Some(read_start), Some(finish_block)) => Some(finish_block - read_start),
			_ => None,
		}
	}

	pub fn total_execution_duration(&self) -> Option<u64> {
		self.finish_block.map(|finish_block| finish_block - self.start_block)
	}
}
