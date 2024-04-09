use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall, SolConstructor};
use anyhow::{Context, Result};
use indicatif::ProgressBar;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{AtBlock, CallContract, CallResult, SubmitResult};
use sp_core::crypto::Ss58Codec;
use std::collections::HashMap;
use std::future::Future;
use std::intrinsics::transmute;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tabled::{builder::Builder, settings::Style};
use tc_subxt::ext::futures::future::join_all;
use tc_subxt::ext::futures::stream::BoxStream;
use tc_subxt::ext::futures::{stream, StreamExt};
use tc_subxt::timechain_runtime::tasks::events::{GatewayRegistered, TaskCreated};
use tc_subxt::{SubxtClient, SubxtTxSubmitter};
use time_primitives::sp_core::H160;
use time_primitives::{
	sp_core, BlockHash, BlockNumber, Function, IGateway, Msg, NetworkId, Runtime, ShardId,
	TaskDescriptor, TaskDescriptorParams, TaskId, TaskPhase, TssKey, TssPublicKey,
};
use tokio::time::Instant;

// type for eth contract address
pub type EthContractAddress = [u8; 20];

// A fixed gas cost of executing the gateway contract
pub const GATEWAY_EXECUTE_GAS_COST: u128 = 100_000;

pub async fn sleep_or_abort(duration: Duration) -> Result<()> {
	tokio::select! {
		_ = tokio::signal::ctrl_c() => {
			println!("aborting...");
			anyhow::bail!("abort");
		},
		_ = tokio::time::sleep(duration) => Ok(()),
	}
}

#[derive(Clone, Debug)]
pub struct Network {
	pub id: NetworkId,
	pub url: String,
}

impl std::str::FromStr for Network {
	type Err = anyhow::Error;

	fn from_str(network: &str) -> Result<Self> {
		let (id, url) = network.split_once(';').context(
			"invalid network, expected network id followed by a semicolon followed by the rpc url",
		)?;
		Ok(Self {
			id: id.parse()?,
			url: url.into(),
		})
	}
}

pub struct Tester {
	network_id: NetworkId,
	gateway_contract: PathBuf,
	runtime: SubxtClient,
	wallet: Wallet,
}

type TssKeyR = <TssKey as alloy_sol_types::SolType>::RustType;

pub async fn subxt_client(keyfile: &Path, url: &str) -> Result<SubxtClient> {
	while SubxtClient::get_client(url).await.is_err() {
		println!("waiting for chain to start");
		sleep_or_abort(Duration::from_secs(10)).await?;
	}

	let tx_submitter = SubxtTxSubmitter::try_new(url).await.unwrap();
	let runtime = SubxtClient::with_keyfile(url, keyfile, tx_submitter).await?;
	println!("tester key is {:?}", runtime.account_id().to_ss58check());
	Ok(runtime)
}

impl Tester {
	pub async fn new(
		runtime: SubxtClient,
		network: &Network,
		keyfile: &Path,
		gateway: &Path,
	) -> Result<Self> {
		let (conn_blockchain, conn_network) = runtime
			.get_network(network.id)
			.await?
			.ok_or(anyhow::anyhow!("Unknown network id"))?;

		let wallet =
			Wallet::new(conn_blockchain.parse()?, &conn_network, &network.url, Some(keyfile), None)
				.await?;
		Ok(Self {
			network_id: network.id,
			gateway_contract: gateway.into(),
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
		// TODO: Calculate the gas_limit necessary to execute the test, then replace this
		// by: gas_limit * gas_price, where gas_price changes depending on the network
		let balance = match self.network_id {
			6 => 10u128.pow(25), // astar
			3 => 10u128.pow(29), // ethereum
			network_id => {
				println!("network id: {network_id} not compatible for faucet");
				return;
			},
		};
		if let Err(err) = self.wallet.faucet(balance).await {
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

		println!("Deploy contract address {contract_address:?} on {block_number:?}");
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
		let events = self.runtime.create_task(params).await?.wait_for_success().await?;
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
			.wait_for_success()
			.await?;
		let gateway_event = events.find_first::<GatewayRegistered>().unwrap();
		println!("Gateway registered with event {:?}", gateway_event);
		Ok(())
	}

	pub async fn is_task_finished(&self, task_id: TaskId) -> bool {
		self.runtime.is_task_complete(task_id).await.unwrap()
	}

	pub async fn get_task(&self, task_id: TaskId) -> TaskDescriptor {
		// block hash is not used so any hash doesnt matter
		self.runtime
			.get_task(Default::default(), task_id)
			.await
			.unwrap()
			.unwrap_or_else(|| panic!("Task not found for task_id {:?}", task_id))
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
			println!("looking for gateway");
			if let Some(gateway) = self.runtime.get_gateway(self.network_id).await? {
				println!("Gateway contract already deployed at {:?}. If you want to redeploy, please use the --redeploy flag.", H160::from_slice(&gateway[..]));
				return Ok(gateway);
			}
		}
		let shard_id = self.wait_for_shard().await?;
		let shard_public_key = self.runtime.shard_public_key(shard_id).await.unwrap();
		let (address, block_height) = self.deploy_gateway(shard_public_key).await?;
		self.register_gateway_address(shard_id, address, block_height).await?;
		// can you believe it, substrate can return old values after emitting a
		// successful event
		tokio::time::sleep(Duration::from_secs(20)).await;
		Ok(address)
	}

	pub async fn get_latest_block(&self) -> Result<u64> {
		self.runtime.get_latest_block().await
	}

	pub async fn finality_block_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.runtime.finality_notification_stream()
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

#[derive(Debug)]
pub struct GmpBenchState {
	total_calls: u64,
	total_deposit: u128,
	gmp_start_time: Instant,
	gmp_execution_duration: Duration,
	pub tasks: HashMap<TaskId, TaskPhaseInfo>,
	total_src_gas: Vec<u128>,
}

impl GmpBenchState {
	pub fn new(total_calls: u64) -> Self {
		Self {
			total_calls,
			gmp_start_time: Instant::now(),
			gmp_execution_duration: Duration::from_secs(0),
			total_deposit: Default::default(),
			tasks: HashMap::with_capacity(total_calls as usize),
			total_src_gas: Vec::with_capacity(total_calls as usize),
		}
	}

	pub fn set_deposit(&mut self, deposit: u128) {
		self.total_deposit = deposit;
	}

	pub fn start(&mut self) {
		self.gmp_start_time = Instant::now();
	}

	pub fn finish(&mut self) {
		self.gmp_execution_duration = Instant::now().duration_since(self.gmp_start_time);
	}

	pub fn get_start_time(&self) -> Instant {
		self.gmp_start_time
	}

	pub fn current_duration(&self) -> Duration {
		Instant::now().duration_since(self.gmp_start_time)
	}

	pub fn insert_src_gas(&mut self, src_gas: Vec<u128>) {
		self.total_src_gas.extend(src_gas);
	}

	pub fn dest_src_gas(&mut self, dest_gas: u128) {
		self.total_deposit = dest_gas;
	}

	pub fn add_task(&mut self, task_id: u64) {
		self.tasks.insert(task_id, TaskPhaseInfo::new());
	}

	pub fn task_ids(&self) -> Vec<TaskId> {
		self.tasks.keys().cloned().collect()
	}

	pub fn finish_task(&mut self, task_id: TaskId) {
		let phase = self.tasks.get_mut(&task_id);
		if let Some(phase) = phase {
			phase.task_finished();
		}
	}

	pub async fn sync_phase(&mut self, src_tester: &Tester) {
		for (task_id, phase) in self.tasks.iter_mut() {
			let task_state = src_tester.get_task_phase(*task_id).await;
			phase.shift_phase(task_state)
		}
	}

	pub fn print_analysis(&self) {
		let mut builder = Builder::new();
		builder.push_record([
			"task_id",
			"sign_phase_duration",
			"write_phase_duration",
			"read_phase_duration",
		]);

		// Using fold to safely handle potential overflow
		let total_src_gas_used = self.total_src_gas.iter().fold(0u128, |acc, &x| {
			acc.checked_add(x)
				.unwrap_or_else(|| panic!("Cannot sum total src fee number overflowed!"))
		});

		// total time since the execution start
		let total_spent_time = self.gmp_execution_duration;

		// calculate task duration for each task
		let all_task_phase_duration: Vec<_> = self
			.tasks
			.iter()
			.map(|(k, v)| {
				(
					k,
					v.start_to_write_duration().unwrap().as_secs_f32(),
					v.write_to_read_duration().unwrap().as_secs_f32(),
					v.read_to_finish_duration().unwrap().as_secs_f32(),
				)
			})
			.collect();

		for item in all_task_phase_duration.clone() {
			builder.push_record([
				format!("{}", item.0),
				format!("{}", item.1),
				format!("{}", item.2),
				format!("{}", item.3),
			]);
		}

		let table = builder.build().with(Style::ascii_rounded()).to_string();
		println!("task phase details \n{}", table);

		// print task phase
		println!("task phase details: {:?}", all_task_phase_duration);

		// print task latencies
		self.print_latencies();

		// print src contract total gas
		println!(
			"Total gas given for src contract calls for {:?} tasks is {:?}",
			self.total_calls, total_src_gas_used
		);

		// print time taken for gmp tasks
		println!(
			"Time taken for {:?} gmp tasks is {:?}",
			self.total_calls,
			format_duration(total_spent_time)
		);
	}

	fn print_latencies(&self) {
		let write_latencies: Vec<Duration> = self
			.tasks
			.values()
			.map(|info| info.start_to_write_duration().unwrap())
			.collect();

		let read_latencies: Vec<Duration> =
			self.tasks.values().map(|info| info.write_to_read_duration().unwrap()).collect();

		let finish_latencies: Vec<Duration> = self
			.tasks
			.values()
			.map(|info| info.read_to_finish_duration().unwrap())
			.collect();

		let total_latencies: Vec<Duration> = self
			.tasks
			.values()
			.map(|info| info.total_execution_duration().unwrap())
			.collect();

		let sum_duration = |duration: Vec<Duration>| {
			duration.iter().fold(Duration::new(0, 0), |acc, &dur| acc + dur)
		};

		let average_write_latency =
			sum_duration(write_latencies.clone()) / write_latencies.len() as u32;
		let average_read_latency =
			sum_duration(read_latencies.clone()) / read_latencies.len() as u32;
		let average_finish_latency =
			sum_duration(finish_latencies.clone()) / finish_latencies.len() as u32;
		let average_total_latency =
			sum_duration(total_latencies.clone()) / total_latencies.len() as u32;

		// let throughput = self.total_calls as f32 / self.gmp_execution_duration.as_secs_f32();

		println!(
			"Average write phase latency is {} secs per task",
			average_write_latency.as_secs()
		);
		println!("Average read phase latency is {} secs per task", average_read_latency.as_secs());
		println!(
			"Average finish phase latency is {} secs per task",
			average_finish_latency.as_secs()
		);
		println!("Average total latency is {} secs per task", average_total_latency.as_secs());
	}

	pub fn get_finished_tasks(&self) -> usize {
		self.tasks.iter().filter(|(_, phase)| phase.finish_time.is_some()).count()
	}
}

#[derive(Debug)]
pub struct TaskPhaseInfo {
	pub start_time: Instant,
	pub write_phase_start: Option<Instant>,
	pub read_phase_start: Option<Instant>,
	pub finish_time: Option<Instant>,
}

impl Default for TaskPhaseInfo {
	fn default() -> Self {
		Self::new()
	}
}

impl TaskPhaseInfo {
	pub fn new() -> Self {
		Self {
			start_time: Instant::now(),
			write_phase_start: None,
			read_phase_start: None,
			finish_time: None,
		}
	}

	pub fn shift_phase(&mut self, phase: TaskPhase) {
		if self.finish_time.is_none() {
			match phase {
				// task start time is task sign phase
				TaskPhase::Sign => {},
				TaskPhase::Write => self.enter_write_phase(),
				TaskPhase::Read => self.enter_read_phase(),
			}
		}
	}

	pub fn enter_write_phase(&mut self) {
		if self.write_phase_start.is_none() {
			self.write_phase_start = Some(Instant::now());
		}
	}

	pub fn enter_read_phase(&mut self) {
		if self.read_phase_start.is_none() {
			self.read_phase_start = Some(Instant::now());
		}
	}

	pub fn task_finished(&mut self) {
		if self.finish_time.is_none() {
			self.finish_time = Some(Instant::now());
		}
	}

	pub fn start_to_write_duration(&self) -> Option<Duration> {
		self.write_phase_start
			.map(|write_start| write_start.duration_since(self.start_time))
	}

	pub fn write_to_read_duration(&self) -> Option<Duration> {
		match (self.write_phase_start, self.read_phase_start) {
			(Some(write_start), Some(read_start)) => Some(read_start.duration_since(write_start)),
			_ => None,
		}
	}

	pub fn read_to_finish_duration(&self) -> Option<Duration> {
		match (self.read_phase_start, self.finish_time) {
			(Some(read_start), Some(finish_block)) => Some(finish_block - read_start),
			_ => None,
		}
	}

	pub fn total_execution_duration(&self) -> Option<Duration> {
		self.finish_time
			.map(|finish_block| finish_block.duration_since(self.start_time))
	}
}

pub async fn test_setup(
	tester: &Tester,
	contract: &Path,
) -> Result<(EthContractAddress, EthContractAddress, u64)> {
	tester.faucet().await;
	let gmp_contract = tester.setup_gmp(false).await?;
	let (contract, start_block) = tester
		.deploy(contract, VotingContract::constructorCall { _gateway: gmp_contract.into() })
		.await?;
	tester
		.wallet()
		.eth_send_call(
			contract,
			VotingContract::registerGmpContractsCall { _registered: vec![] }.abi_encode(),
			0,
			None,
			None,
		)
		.await?;
	Ok((gmp_contract, contract, start_block))
}

///
/// fetches the testcontract state (contains yes and no votes and gets total of each)
pub async fn stats(
	tester: &Tester,
	contract: EthContractAddress,
	block: Option<u64>,
) -> Result<(u64, u64)> {
	let block: AtBlock = if let Some(block) = block { block.into() } else { AtBlock::Latest };
	let call = VotingContract::statsCall {};
	let stats = tester.wallet().eth_view_call(contract, call.abi_encode(), block).await?;
	let CallResult::Success(stats) = stats else { anyhow::bail!("{:?}", stats) };
	if stats.is_empty() {
		println!("Stats are empty returning default on block {:?}", block);
		return Ok((0, 0));
	}
	let stats = VotingContract::statsCall::abi_decode_returns(&stats, true)?._0;
	let yes_votes = stats[0].try_into().unwrap();
	let no_votes = stats[1].try_into().unwrap();
	Ok((yes_votes, no_votes))
}

///
/// Sets up contract on destination and source chain
///
/// # Arguments
/// `src`: Tester connected to first network
/// `dest`: Tester connected to second network
/// `contract`: Contract path of test contract
/// `total_calls`: gas funding for total number of calls
pub async fn setup_gmp_with_contracts(
	src: &Tester,
	dest: &Tester,
	contract: &Path,
	total_calls: u128,
) -> Result<(EthContractAddress, EthContractAddress, u128)> {
	src.faucet().await;
	dest.faucet().await;
	let src_gmp_contract = src.setup_gmp(false).await?;
	let dest_gmp_contract = dest.setup_gmp(false).await?;

	// deploy testing contract for source chain
	let (src_contract, _) = src
		.deploy(
			contract,
			VotingContract::constructorCall {
				_gateway: src_gmp_contract.into(),
			},
		)
		.await?;

	// deploy testing contract for destination/target chain
	let (dest_contract, _) = dest
		.deploy(
			contract,
			VotingContract::constructorCall {
				_gateway: dest_gmp_contract.into(),
			},
		)
		.await?;

	let src_network = src.network_id();
	let dest_network = dest.network_id();

	// registers destination contract in source contract to inform gmp compatibility.
	src.wallet()
		.eth_send_call(
			src_contract,
			VotingContract::registerGmpContractsCall {
				_registered: vec![GmpVotingContract {
					dest: dest_contract.into(),
					network: dest_network,
				}],
			}
			.abi_encode(),
			0,
			None,
			None,
		)
		.await?;

	// registers source contract in destination contract to inform gmp compatibility.
	dest.wallet()
		.eth_send_call(
			dest_contract,
			VotingContract::registerGmpContractsCall {
				_registered: vec![GmpVotingContract {
					dest: src_contract.into(),
					network: src_network,
				}],
			}
			.abi_encode(),
			0,
			None,
			None,
		)
		.await?;

	let deposit_amount =
		deposit_gmp_funds(src, src_contract, dest, dest_contract, total_calls).await?;

	Ok((src_contract, dest_contract, deposit_amount))
}

///
/// gmp setup if the test and gateway contract is already deployed
///
/// # Argument
/// `contracts`: (src_contract, destination_contract) in respective chains
/// `src`: tester connected to source chain
/// `dest`: tester connected to destination chain
/// `total_calls`: total number of calls
pub async fn setup_funds_if_needed(
	contracts: (String, String),
	src: &Tester,
	dest: &Tester,
	total_calls: u128,
) -> Result<(EthContractAddress, EthContractAddress, u128)> {
	// if contracts already provided then get
	let mut src_contract: EthContractAddress = [0; 20];
	let mut dest_contract: EthContractAddress = [0; 20];
	src_contract.copy_from_slice(
		&hex::decode(contracts.0.strip_prefix("0x").unwrap_or(&contracts.0)).unwrap()[..20],
	);
	dest_contract.copy_from_slice(
		&hex::decode(contracts.1.strip_prefix("0x").unwrap_or(&contracts.1)).unwrap()[..20],
	);

	let deposit_amount =
		deposit_gmp_funds(src, src_contract, dest, dest_contract, total_calls).await?;

	Ok((src_contract, dest_contract, deposit_amount))
}

///
/// Wait for src contract gmp calls in chunks
///
/// # Argument
/// `src`: tester connected to source chain
/// `src_contract`: contract in source chain that implements IGMPReceiver
/// `dest`: tester connected to destination chain
/// `dest_contract`: contract in destination chain that implements IGMPReceiver interface
/// `total_calls`: total number of calls to deposit funds for
pub async fn deposit_gmp_funds(
	src: &Tester,
	src_contract: EthContractAddress,
	dest: &Tester,
	dest_contract: EthContractAddress,
	total_calls: u128,
) -> Result<u128> {
	let dest_gmp_contract = dest.runtime.get_gateway(dest.network_id).await.unwrap().unwrap();
	let src_network = src.network_id();

	// calculate how many funds do we have in contract
	let funds_in_contract = {
		let mut deposit_src = [0; 32];
		deposit_src[11] = 1;
		deposit_src[12..32].copy_from_slice(&src_contract[..]);

		let call = IGateway::depositOfCall {
			source: deposit_src.into(),
			networkId: src_network,
		}
		.abi_encode();

		let result = dest.wallet().eth_view_call(dest_gmp_contract, call, AtBlock::Latest).await?;
		match result {
			CallResult::Success(payload) => {
				u128::try_from(alloy_primitives::U256::from_be_slice(&payload)).unwrap()
			},
			_ => anyhow::bail!("Failed to get GMP_GAS_LIMIT: {result:?}"),
		}
	};

	// this is reregisteration source contract in destination contract to inform gmp compatibility.
	let receipt = dest
		.wallet()
		.eth_send_call(
			dest_contract,
			VotingContract::registerGmpContractsCall {
				_registered: vec![GmpVotingContract {
					dest: src_contract.into(),
					network: src_network,
				}],
			}
			.abi_encode(),
			0,
			None,
			None,
		)
		.await?
		.receipt()
		.unwrap()
		.clone();

	// Calculate the gas price based on the latest transaction
	let gas_price = u128::try_from(receipt.effective_gas_price.unwrap()).unwrap();

	// Get the GMP_GAS_LIMIT from the VotingContract
	let gmp_gas_limit = {
		let result = dest
			.wallet()
			.query(CallContract {
				from: None,
				to: src_contract.into(),
				value: 0.into(),
				data: VotingContract::GMP_GAS_LIMITCall {}.abi_encode(),
				block: AtBlock::Latest,
			})
			.await?;
		match result {
			CallResult::Success(payload) => {
				u128::try_from(alloy_primitives::U256::from_be_slice(&payload)).unwrap()
			},
			_ => anyhow::bail!("Failed to get GMP_GAS_LIMIT: {result:?}"),
		}
	};

	// Calculate the deposit amount based on the gas_cost x gas_price + 20%
	//how much the user provides the gmp
	let gmp_gas_limit = gmp_gas_limit.saturating_add(GATEWAY_EXECUTE_GAS_COST);
	let deposit_amount = gas_price
		.saturating_mul(gmp_gas_limit)
		.saturating_mul(12)
		.saturating_mul(total_calls)
		/ 10; // 20% more, in case of the gas price increase

	let remaining_submitable_gas = deposit_amount.saturating_sub(funds_in_contract);
	println!("funds required for call:     {:?}", deposit_amount);
	println!("funds available in contract: {:?}", funds_in_contract);
	println!("depositing in contract:      {:?}", remaining_submitable_gas);
	println!("balance in wallet            {:?}", dest.wallet.balance().await?);

	//check if wallet have enough funds to deposit funds
	assert!(dest.wallet.balance().await? > remaining_submitable_gas);

	if remaining_submitable_gas > 0 {
		// deposit funds for source in gmp contract to be able to execute the call
		dest.deposit_funds(dest_gmp_contract, src_network, src_contract, true, deposit_amount)
			.await?;
	}

	Ok(deposit_amount)
}

///
/// Wait for src contract gmp calls in chunks
///
/// # Argument
/// `calls`: futures contianing gmp calls to src_contract
/// `number_of_calls`: total number of calls for for gmp benchmarks
/// `chunks`: chunks to wait for in a single turn
pub async fn wait_for_gmp_calls<I, F>(
	calls: I,
	number_of_calls: u64,
	chunk_size: usize,
) -> Result<Vec<SubmitResult>>
where
	I: IntoIterator<Item = F>,
	F: Future<Output = Result<SubmitResult, anyhow::Error>>,
{
	let mut results = Vec::new();

	let pb = ProgressBar::new(number_of_calls);
	let calls_stream = stream::iter(calls.into_iter());
	let mut chunks = calls_stream.chunks(chunk_size);

	while let Some(chunk) = chunks.next().await {
		println!("Waiting for gmp init calls");
		let futures = chunk.into_iter().collect::<Vec<_>>();
		let chunk_results: Result<Vec<_>, _> = join_all(futures).await.into_iter().collect();
		results.extend(chunk_results?);
		pb.inc(chunk_size as u64);
	}
	pb.finish_with_message("all src tx successful..");

	Ok(results)
}

///
/// Format duration into proper time format
pub fn format_duration(duration: Duration) -> String {
	let seconds = duration.as_secs();
	let hours = seconds / 3600;
	let minutes = (seconds % 3600) / 60;
	let secs = seconds % 60;

	format!("{:02} hrs {:02} mins {:02} secs", hours, minutes, secs)
}
