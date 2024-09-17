use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolCall, SolConstructor};
use anyhow::{Context, Result};
use num_bigint::BigUint;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{AtBlock, CallResult, GetTransactionCount, SubmitResult};
use schnorr_evm::SigningKey;
use sol::{
	Gateway, GatewayProxy, GmpVotingContract, Network, TssKey, UpdateNetworkInfo, VotingContract,
};
use std::collections::{BTreeSet, HashMap};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;
use tabled::{builder::Builder, settings::Style};
use tc_subxt::ext::futures::future::join_all;
use tc_subxt::ext::futures::stream::BoxStream;
use tc_subxt::ext::futures::{stream, StreamExt};
use tc_subxt::{events, MetadataVariant, SubxtClient, SubxtTxSubmitter};
use time_primitives::traits::Ss58Codec;
use time_primitives::{
	BlockHash, BlockNumber, Function, GmpParams, IGateway, Message, Msg, NetworkId, Runtime,
	ShardId, ShardStatus, TaskDescriptor, TaskDescriptorParams, TaskId, TaskPhase, TssPublicKey,
	H160,
};
use tokio::time::Instant;
use ufloat::{Float, Rounding, UFloat9x56};
pub mod sol;
mod ufloat;

// type for eth contract address
pub type EthContractAddress = [u8; 20];

type TssKeyR = <TssKey as alloy_sol_types::SolType>::RustType;

// A fixed gas cost of executing the gateway contract
pub const GATEWAY_EXECUTE_GAS_COST: u128 = 100_000;
pub const GATEWAY_BASE_GAS_COST: u128 = 100_000;

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
pub struct ChainNetwork {
	pub id: NetworkId,
	pub url: String,
}

impl FromStr for ChainNetwork {
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
	proxy_contract: PathBuf,
	runtime: SubxtClient,
	wallet: Wallet,
}

pub async fn subxt_client(
	keyfile: &Path,
	metadata: MetadataVariant,
	url: &str,
) -> Result<SubxtClient> {
	while SubxtClient::get_client(url).await.is_err() {
		println!("waiting for chain to start");
		sleep_or_abort(Duration::from_secs(10)).await?;
	}

	let tx_submitter = SubxtTxSubmitter::try_new(url).await.unwrap();
	let runtime = SubxtClient::with_keyfile(url, metadata, keyfile, tx_submitter).await?;
	println!("tester key is {:?}", runtime.account_id().to_ss58check());
	Ok(runtime)
}

impl Tester {
	pub async fn new(
		runtime: SubxtClient,
		network: &ChainNetwork,
		keyfile: &Path,
		gateway: &Path,
		proxy: &Path,
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
			proxy_contract: proxy.into(),
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

	pub async fn gateway(&self) -> Result<Option<EthContractAddress>> {
		self.runtime.get_gateway(self.network_id).await
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
	) -> Result<(EthContractAddress, u64)> {
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

	pub async fn deploy_gateway(&self, proxy: Address) -> Result<(EthContractAddress, u64)> {
		let call = IGateway::constructorCall {
			networkId: self.network_id,
			proxy,
		};
		self.deploy(&self.gateway_contract, call).await
	}

	pub async fn deploy_proxy(
		&self,
		implementation: Address,
		proxy_addr: Address,
		admin: Address,
		tss_public_key: Vec<TssPublicKey>,
		networks: Vec<Network>,
	) -> Result<(EthContractAddress, u64)> {
		// Build the Gateway initializer
		let tss_keys: Vec<TssKeyR> = tss_public_key
			.into_iter()
			.map(|key| {
				let parity_bit = if key[0] % 2 == 0 { 0 } else { 1 };
				let x_coords = hex::encode(&key[1..]);
				TssKeyR {
					yParity: parity_bit,
					xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
				}
			})
			.collect();
		let initializer = Gateway::initializeCall {
			admin: admin.into_array().into(),
			keys: tss_keys,
			networks,
		}
		.abi_encode();

		// Deploy the proxy contract
		let call = GatewayProxy::constructorCall {
			implementation,
			initializer: initializer.into(),
		};
		let (actual_addr, block_number) = self.deploy(&self.proxy_contract, call).await?;

		// Check if the proxy address match the expect address
		let actual_addr: Address = actual_addr.into();
		if actual_addr != proxy_addr.into_array() {
			anyhow::bail!("Proxy address mismatch, expect {proxy_addr:?}, got {actual_addr:?}");
		}
		Ok((actual_addr.into_array(), block_number))
	}

	pub async fn deposit_gateway_funds(
		&self,
		gmp_address: EthContractAddress,
		amount: u128,
	) -> Result<()> {
		let payload = IGateway::depositCall {}.abi_encode();
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

	pub async fn shard_status(&self, shard_id: u64) -> Result<ShardStatus> {
		self.runtime.shard_state(shard_id).await
	}

	pub async fn is_shard_online(&self, shard_id: u64) -> Result<bool> {
		Ok(self.shard_status(shard_id).await? == ShardStatus::Online)
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
		while !self.is_shard_online(shard_id).await? {
			println!("Waiting for shard to go online");
			sleep_or_abort(Duration::from_secs(10)).await?;
		}
		Ok(shard_id)
	}

	pub async fn shard_size(&self) -> Result<u16> {
		self.runtime.shard_size().await
	}

	pub async fn shard_threshold(&self) -> Result<u16> {
		self.runtime.shard_threshold().await
	}

	pub async fn set_shard_config(&self, shard_size: u16, shard_threshold: u16) -> Result<()> {
		self.runtime
			.set_shard_config(shard_size, shard_threshold)
			.await?
			.wait_for_success()
			.await?;
		Ok(())
	}

	pub async fn register_network(&self, chain_name: String, chain_network: String) -> Result<()> {
		self.runtime
			.register_network(chain_name, chain_network)
			.await?
			.wait_for_success()
			.await?;
		Ok(())
	}

	pub async fn create_task(&self, function: Function, block: u64) -> Result<TaskId> {
		println!("creating task");
		let shard_size = self.shard_size().await?;
		let params = TaskDescriptorParams {
			network: self.network_id,
			function,
			start: block,
			shard_size,
			funds: 10000000000000000,
		};
		let events = self.runtime.create_task(params).await?.wait_for_success().await?;
		let transfer_event = events.find_first::<events::TaskCreated>().unwrap();
		let events::TaskCreated(id) =
			transfer_event.ok_or(anyhow::anyhow!("Not able to fetch task event"))?;
		println!("Task registered: {:?}", id);
		Ok(id)
	}

	pub async fn register_gateway_address(
		&self,
		shard_id: u64,
		address: EthContractAddress,
		block_height: u64,
	) -> Result<()> {
		let events = self
			.runtime
			.register_gateway(shard_id, address, block_height)
			.await?
			.wait_for_success()
			.await?;
		let gateway_event = events.find_first::<events::GatewayRegistered>().unwrap();
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

	pub async fn get_task_phase(&self, task_id: TaskId) -> Option<TaskPhase> {
		self.runtime.get_task_phase(task_id).await.unwrap()
	}

	pub async fn get_network_unassigned_tasks(&self, network_id: u16) -> Vec<TaskId> {
		self.runtime.get_network_unassigned_tasks(network_id).await.unwrap()
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

	pub async fn setup_gmp(
		&self,
		redeploy: bool,
		keyfile: Option<PathBuf>,
		networks: Vec<Network>,
	) -> Result<EthContractAddress> {
		let mut gateway_keys: Vec<TssPublicKey> = vec![];
		if let Some(file) = keyfile {
			let bytes = std::fs::read_to_string(file)?;
			let key: Vec<u8> = serde_json::from_str(&bytes)?;
			let schnorr_signing_key =
				SigningKey::from_bytes(key.try_into().expect("Invalid secret key provided"))?;
			let public_key = schnorr_signing_key.public().to_bytes()?;
			gateway_keys.push(public_key);
		}

		if !redeploy {
			println!("looking for gateway against network id: {:?}", self.network_id);
			if let Some(gateway) = self.runtime.get_gateway(self.network_id).await? {
				println!("Gateway contract already deployed at {:?}. If you want to redeploy, please use the --redeploy flag.", H160::from_slice(&gateway[..]));
				return Ok(gateway);
			}
		}
		let shard_id = self.wait_for_shard().await?;
		let shard_public_key = self.runtime.shard_public_key(shard_id).await.unwrap();
		gateway_keys.push(shard_public_key);

		// get proxy and admin addresses
		let proxy_addr = self.get_proxy_addr().await?;
		let gateway_admin = Address::from_str(self.wallet().account().address.as_str())?;

		// deploy gateway implementation
		println!("deploying implementation contract...");
		let (address, _) = self.deploy_gateway(proxy_addr).await?;

		// deploy and initialize gateway proxy
		println!("deploying proxy contract...");
		let (proxy_addr, block_height) = self
			.deploy_proxy(address.into(), proxy_addr, gateway_admin, gateway_keys, networks.clone())
			.await?;

		// register proxy
		self.register_gateway_address(shard_id, proxy_addr, block_height).await?;
		println!("registering network on gateway");
		// can you believe it, substrate can return old values after emitting a
		// successful event
		tokio::time::sleep(Duration::from_secs(20)).await;

		if !networks.is_empty() {
			let src_network = networks.first().unwrap().id;
			let dest_network = networks.last().unwrap().id;

			let (num, den) = get_network_ratio(src_network, dest_network);

			self.set_network_info(num.into(), den.into(), dest_network, 1_000_000, 100_000)
				.await?;
		}
		Ok(proxy_addr)
	}

	pub async fn get_proxy_addr(&self) -> Result<Address> {
		// account which is going to deploy the gateway
		let deployer = Address::from_str(self.wallet().account().address.as_str())?;

		// get deployer's nonce
		let nonce = self
			.wallet()
			.query(GetTransactionCount {
				address: deployer.into_array().into(),
				block: AtBlock::Latest,
			})
			.await?;

		// compute the proxy address
		Ok(deployer.create(nonce + 1))
	}

	pub async fn register_shard_on_gateway(
		&self,
		shard_id: ShardId,
		keyfile: PathBuf,
	) -> Result<()> {
		// schnorr signing key
		let bytes = std::fs::read_to_string(keyfile)?;
		let key: Vec<u8> = serde_json::from_str(&bytes)?;
		let schnorr_signing_key =
			SigningKey::from_bytes(key.try_into().expect("Invalid secret key provided"))?;
		let public_key = schnorr_signing_key.public().to_bytes()?;

		// msg building
		// provides signer who is gonna sign to gmp_params i.e. must be registered in gateway contract
		let gmp_params = self.get_gmp_params(&public_key).await?;
		// shard commitment
		let shard_public_key = self.runtime.shard_public_key(shard_id).await.unwrap();
		// provides key to register
		let msg = Message::update_keys([], [shard_public_key]);
		// builds signing payload
		let payload = msg.to_eip712_bytes(&gmp_params);
		// sign the msg
		let sig = schnorr_signing_key.sign(&payload).to_bytes();

		let call = msg.into_evm_call(&gmp_params, sig);
		let Function::EvmCall {
			address,
			input,
			amount,
			gas_limit,
		} = call
		else {
			println!("function is not valid");
			return Ok(());
		};
		self.wallet()
			.eth_send_call(address, input.clone(), amount, None, gas_limit)
			.await?;
		println!("Shard sucessfully registered");
		Ok(())
	}

	async fn get_gmp_params(&self, shard_key: &[u8; 33]) -> Result<GmpParams> {
		let gateway = self
			.runtime
			.get_gateway(self.network_id)
			.await?
			.expect("Gateway contract not registered");
		Ok(GmpParams {
			network_id: self.network_id,
			gateway_contract: gateway.into(),
			tss_public_key: *shard_key,
		})
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
		source: EthContractAddress,
		dest: EthContractAddress,
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

	pub async fn gateway_update(&self, proxy_address: Address) -> Result<()> {
		let (gateway_addr, _) = self.deploy_gateway(proxy_address).await?;
		let call = Gateway::upgradeCall {
			newImplementation: gateway_addr.into(),
		}
		.abi_encode();
		println!("call data for gateway update: {:?}", hex::encode(&call));
		let result = self.wallet().eth_send_call(proxy_address.into(), call, 0, None, None).await?;
		match result {
			SubmitResult::Executed { tx_hash, .. } => {
				println!("tx successful: {:?}", tx_hash)
			},
			SubmitResult::Timeout { tx_hash } => {
				println!("tx timedout: {:?}", tx_hash)
			},
		}
		Ok(())
	}
	pub async fn gateway_set_admin(
		&self,
		proxy_address: Address,
		new_admin: Address,
	) -> Result<()> {
		let call = Gateway::setAdminCall { newAdmin: new_admin }.abi_encode();
		println!("call data for set admin: {:?}", hex::encode(&call));
		let result = self.wallet().eth_send_call(proxy_address.into(), call, 0, None, None).await?;
		match result {
			SubmitResult::Executed { tx_hash, .. } => {
				println!("tx successful: {:?}", tx_hash)
			},
			SubmitResult::Timeout { tx_hash } => {
				println!("tx timedout: {:?}", tx_hash)
			},
		}
		Ok(())
	}
	pub async fn gateway_add_shards(&self, shard_ids: Vec<ShardId>) -> Result<()> {
		let Some(proxy) = self.runtime.get_gateway(self.network_id).await? else {
			panic!("No proxy deployed for this network");
		};
		let mut tss_keys: Vec<TssKeyR> = vec![];
		for id in shard_ids {
			let key = self.runtime.shard_public_key(id).await?;
			let parity_bit = if key[0] % 2 == 0 { 0 } else { 1 };
			let x_coords = hex::encode(&key[1..]);
			tss_keys.push(TssKeyR {
				yParity: parity_bit,
				xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
			});
		}
		let call = Gateway::sudoAddShardsCall { shards: tss_keys }.abi_encode();
		println!("call data for add shards: {:?}", hex::encode(&call));
		let result = self.wallet().eth_send_call(proxy, call, 0, None, None).await?;
		match result {
			SubmitResult::Executed { tx_hash, .. } => {
				println!("tx successful: {:?}", tx_hash)
			},
			SubmitResult::Timeout { tx_hash } => {
				println!("tx timedout: {:?}", tx_hash)
			},
		}
		Ok(())
	}

	pub async fn set_network_info(
		&self,
		numerator: BigUint,
		denominator: BigUint,
		network_id: NetworkId,
		gas_limit: u64,
		base_fee: u128,
	) -> Result<()> {
		let gateway = self.gateway().await?.expect("Gateway address not found");
		let float =
			UFloat9x56::rational_to_float(numerator, denominator, Rounding::NearestTiesEven)
				.map_err(|e| anyhow::anyhow!("{:?}", e))?;
		let gmp_params = GmpParams {
			network_id,
			gateway_contract: gateway.into(),
			// doesnt matter when computing domain seperator
			tss_public_key: [0; 33],
		};
		let call = Gateway::setNetworkInfoCall {
			info: UpdateNetworkInfo {
				networkId: network_id,
				domainSeparator: gmp_params.eip712_domain_separator().hash_struct(),
				gasLimit: gas_limit,
				relativeGasPrice: float,
				baseFee: base_fee,
				mortality: u64::MAX,
			},
		}
		.abi_encode();
		self.wallet().eth_send_call(gateway, call, 0, None, None).await?;
		Ok(())
	}

	pub async fn submit_signed_payload(&self, task_id: TaskId) -> Result<()> {
		let phase = self.runtime.get_task_phase(task_id).await?.expect("Task not found");
		let TaskPhase::Write = phase else {
			anyhow::bail!("Wrong task phase");
		};
		let shard_id = self.runtime.get_task_shard(task_id).await?.expect("Task not assigned");
		let shard_status = self.shard_status(shard_id).await?;
		if ShardStatus::Offline == shard_status {
			tracing::warn!("Warning! Shard is offline");
		}
		let task = self.get_task(task_id).await;
		let gmp_params = self.build_gmp_params(shard_id).await?;
		let task_signature =
			self.runtime.get_task_signature(task_id).await?.expect("Task not signed");
		let function = match task.function {
			Function::RegisterShard { shard_id } => {
				let shard_key = self.runtime.shard_public_key(shard_id).await?;
				Message::update_keys([], [shard_key]).into_evm_call(&gmp_params, task_signature)
			},
			Function::UnregisterShard { shard_id } => {
				let shard_key = self.runtime.shard_public_key(shard_id).await?;
				Message::update_keys([shard_key], []).into_evm_call(&gmp_params, task_signature)
			},
			Function::SendMessage { msg } => {
				Message::gmp(msg).into_evm_call(&gmp_params, task_signature)
			},
			_ => {
				anyhow::bail!("Call not supported");
			},
		};
		let Function::EvmCall {
			address,
			input,
			amount,
			gas_limit,
		} = function
		else {
			anyhow::bail!("Only EvmCall functions are supported");
		};
		let result = self
			.wallet()
			.eth_send_call(address, input.clone(), amount, None, gas_limit)
			.await?;
		println!("Tx result: {:?}", result);
		Ok(())
	}

	pub async fn build_gmp_params(&self, shard_id: ShardId) -> Result<GmpParams> {
		let gateway_contract = self.gateway().await?.expect("Unable to get gateway");
		let network_id = self.network_id();
		let tss_public_key = self.runtime.shard_public_key(shard_id).await?;
		let gmp_params = GmpParams {
			network_id,
			gateway_contract: gateway_contract.into(),
			tss_public_key,
		};
		Ok(gmp_params)
	}
}

fn compile_file(path: &Path) -> Result<Vec<u8>> {
	let abi = std::fs::read_to_string(path)?;
	let json_abi: serde_json::Value = serde_json::from_str(&abi)?;
	Ok(hex::decode(json_abi["bytecode"]["object"].as_str().unwrap().replace("0x", ""))?)
}

pub fn stop_node(node_name: String) {
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

pub fn create_evm_call(address: EthContractAddress) -> Function {
	Function::EvmCall {
		address,
		input: VotingContract::vote_wihtout_gmpCall { _vote: true }.abi_encode(),
		amount: 0,
		gas_limit: None,
	}
}

pub fn create_evm_view_call(address: EthContractAddress) -> Function {
	Function::EvmViewCall {
		address,
		input: VotingContract::statsCall {}.abi_encode(),
	}
}

#[derive(Debug)]
pub struct GmpBenchState {
	total_calls: u64,
	gmp_start_time: Instant,
	gmp_execution_duration: Duration,
	pub tasks: HashMap<TaskId, TaskPhaseInfo>,
	recv_tasks: HashMap<TaskId, RecvTaskPhase>,
	errored_tasks: HashMap<TaskId, String>,
	total_src_gas: Vec<u128>,
}

impl GmpBenchState {
	pub fn new(total_calls: u64) -> Self {
		Self {
			total_calls,
			gmp_start_time: Instant::now(),
			gmp_execution_duration: Duration::from_secs(0),
			tasks: HashMap::with_capacity(total_calls as usize),
			recv_tasks: Default::default(),
			errored_tasks: Default::default(),
			total_src_gas: Vec::with_capacity(total_calls as usize),
		}
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

	pub fn add_task(&mut self, task_id: u64) {
		self.tasks.insert(task_id, TaskPhaseInfo::new());
	}

	pub fn add_recv_task(&mut self, task_id: u64) {
		self.recv_tasks.insert(task_id, RecvTaskPhase::new());
	}

	pub fn update_recv_gmp_task(&mut self, task_id: u64, gmp_tasks: u64) {
		let recv_phase = self.recv_tasks.get_mut(&task_id);
		if let Some(phase) = recv_phase {
			phase.update_gmp_tasks(gmp_tasks);
		}
	}

	pub fn task_ids(&self) -> BTreeSet<TaskId> {
		self.tasks.keys().cloned().collect()
	}

	pub fn recv_task_ids(&self) -> Vec<TaskId> {
		self.recv_tasks.keys().cloned().collect()
	}

	pub fn add_errored_tasks(&mut self, task_id: TaskId, reason: String) {
		self.errored_tasks.insert(task_id, reason);
	}

	fn get_success_tasks(&self) -> HashMap<TaskId, TaskPhaseInfo> {
		self.tasks
			.iter()
			.filter_map(|(task_id, task_info)| {
				if !self.errored_tasks.contains_key(task_id) {
					Some((*task_id, task_info.clone()))
				} else {
					None
				}
			})
			.collect::<_>()
	}

	pub fn finish_task(&mut self, task_id: TaskId) {
		let phase = self.tasks.get_mut(&task_id);
		if let Some(phase) = phase {
			phase.task_finished();
		}
		let recv_phase = self.recv_tasks.get_mut(&task_id);
		if let Some(phase) = recv_phase {
			phase.finish_task();
		}
	}

	pub async fn sync_phase(&mut self, dest_tester: &Tester) {
		let unassigned_tasks =
			dest_tester.get_network_unassigned_tasks(dest_tester.network_id()).await;
		// update recv_tasks status
		for (task_id, phase) in self.recv_tasks.iter_mut() {
			// if task is unassigned then skip it
			if unassigned_tasks.contains(task_id) {
				continue;
			}

			// there is check in start task if its already started it doesnt start update time again
			// so calling this again and again is just causes a if statement check to run
			phase.start_task()
		}

		// update SendMessage task status
		for (task_id, phase) in self.tasks.iter_mut() {
			// if task not assigned then skip
			if unassigned_tasks.contains(task_id) {
				continue;
			}

			let task_state = dest_tester.get_task_phase(*task_id).await;

			if let Some(task_state) = task_state {
				phase.shift_phase(task_state)
			} else {
				// Getting task phase of some inserted tasks returns a panic saying phase not found
				// which means that the task isnt inserted at the time of fetching the task phase
				println!("Task phase not found for task: {:?}", task_id);
			}
		}
	}

	///
	/// print average delays find in task execution
	pub fn print_analysis(&self) {
		// there are chances that in local read tasks are inserted before block subscription starts
		// that fetches them in that case we might miss one read message task
		// or if all the tx are read by single message the readmessage could be 0
		if !self.recv_tasks.is_empty() {
			// recv message delays per each task
			self.print_recv_message_analysis();
			// recv message average delays per each task
			self.print_recv_message_latencies();
		}
		// send message delays per each task
		self.print_send_message_analysis();
		// send message delays per each task
		self.print_send_message_task_latencies();
	}

	fn print_recv_message_analysis(&self) {
		let mut builder = Builder::new();
		builder.push_record([
			"task_id",
			"creation_after_start (secs)",
			"unassigned (secs)",
			"finish duration (secs)",
			"Gmp tasks",
		]);

		let data: Vec<_> = self
			.recv_tasks
			.iter()
			.map(|(k, v)| {
				(
					k,
					v.start_time.unwrap().duration_since(self.gmp_start_time).as_secs(),
					v.get_start_duration().unwrap().as_secs(),
					v.get_execution_time().unwrap().as_secs(),
					v.gmp_in_task,
				)
			})
			.collect();

		let mut data = data;
		data.sort_by_key(|&(k, _, _, _, _)| k);

		for item in data.clone() {
			builder.push_record([
				format!("{}", item.0),
				format!("{}", item.1),
				format!("{}", item.2),
				format!("{}", item.3),
				format!("{}", item.4),
			]);
		}

		let table = builder.build().with(Style::ascii_rounded()).to_string();
		println!("recv messages details \n{}", table);
	}

	fn print_recv_message_latencies(&self) {
		let recv_tasks = self.recv_tasks.values();
		let creation_latencies: Vec<Duration> = recv_tasks
			.clone()
			.map(|info| info.start_time.unwrap().duration_since(self.gmp_start_time))
			.collect();
		let start_latencies: Vec<Duration> =
			recv_tasks.clone().map(|info| info.get_start_duration().unwrap()).collect();
		let finish_latencies: Vec<Duration> =
			recv_tasks.clone().map(|info| info.get_execution_time().unwrap()).collect();

		let average_creation_latency =
			sum_duration(creation_latencies.clone()) / creation_latencies.len() as u32;
		let average_start_latency =
			sum_duration(start_latencies.clone()) / start_latencies.len() as u32;
		let average_finish_latency =
			sum_duration(finish_latencies.clone()) / finish_latencies.len() as u32;

		println!(
			"Average creation latency for Recieve Message is {} secs",
			average_creation_latency.as_secs()
		);
		println!(
			"Average start latency for Recieve Message is {} secs",
			average_start_latency.as_secs()
		);
		println!(
			"Average finish latency for Recieve Message is {} secs",
			average_finish_latency.as_secs()
		);
	}

	///
	/// print average delays in send message tasks during benchmark
	fn print_send_message_analysis(&self) {
		if !self.errored_tasks.is_empty() {
			println!("Following tasks failed:");
			println!("{:#?}", self.errored_tasks);
		}

		let tasks = self.get_success_tasks();

		let mut builder = Builder::new();
		builder.push_record([
			"task_id",
			"unassigned (secs)",
			"sign_phase (secs)",
			"write_phase (secs)",
			"read_phase (secs)",
		]);

		// Using fold to safely handle potential overflow
		let total_src_gas_used = self.total_src_gas.iter().fold(0u128, |acc, &x| {
			acc.checked_add(x)
				.unwrap_or_else(|| panic!("Cannot sum total src fee number overflowed!"))
		});

		// total time since the execution start
		let total_spent_time = self.gmp_execution_duration;

		// calculate task duration for each task
		let all_task_phase_duration: Vec<_> = tasks
			.iter()
			.map(|(k, v)| {
				(
					k,
					v.unassigned_time().unwrap().as_secs(),
					v.sign_to_write_duration().unwrap().as_secs(),
					v.write_to_read_duration().unwrap().as_secs(),
					v.read_to_finish_duration().unwrap().as_secs(),
				)
			})
			.collect();

		let mut sorted_all_task_phase_duration = all_task_phase_duration;
		sorted_all_task_phase_duration.sort_by_key(|&(k, _, _, _, _)| k);

		for item in sorted_all_task_phase_duration.clone() {
			builder.push_record([
				format!("{}", item.0),
				format!("{}", item.1),
				format!("{}", item.2),
				format!("{}", item.3),
				format!("{}", item.4),
			]);
		}

		// print task phase
		let table = builder.build().with(Style::ascii_rounded()).to_string();
		println!("Send Message task phase details \n{}", table);

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

	///
	/// print average delays find in task execution
	fn print_send_message_task_latencies(&self) {
		let tasks = self.get_success_tasks();

		if tasks.is_empty() {
			return;
		}

		let mut unassigned_latencies = Vec::with_capacity(tasks.len());
		let mut sign_latencies = Vec::with_capacity(tasks.len());
		let mut write_latencies = Vec::with_capacity(tasks.len());
		let mut read_latencies = Vec::with_capacity(tasks.len());
		let mut total_latencies = Vec::with_capacity(tasks.len());

		for (_, info) in tasks.iter() {
			unassigned_latencies.push(info.unassigned_time().unwrap());
			sign_latencies.push(info.sign_to_write_duration().unwrap());
			write_latencies.push(info.write_to_read_duration().unwrap());
			read_latencies.push(info.read_to_finish_duration().unwrap());
			total_latencies.push(info.total_execution_duration().unwrap());
		}

		let average_unassigned_latency =
			sum_duration(unassigned_latencies.clone()) / unassigned_latencies.len() as u32;
		let average_sign_latency =
			sum_duration(sign_latencies.clone()) / sign_latencies.len() as u32;
		let average_write_latency =
			sum_duration(write_latencies.clone()) / write_latencies.len() as u32;
		let average_read_latency =
			sum_duration(read_latencies.clone()) / read_latencies.len() as u32;
		let average_total_latency =
			sum_duration(total_latencies.clone()) / total_latencies.len() as u32;

		// Average time when tasks were unassigned
		println!(
			"Average unassigned task latency was {} secs per task",
			average_unassigned_latency.as_secs()
		);
		// sign phase is when tss signing happens and task is converted to write phase
		println!("Average sign phase latency is {} secs per task", average_sign_latency.as_secs());
		// write phase is when chain sends calls the contract
		println!(
			"Average write phase latency is {} secs per task",
			average_write_latency.as_secs()
		);
		// read phase is when we read the reciept of tx and finish the task
		println!("Average read phase latency is {} secs per task", average_read_latency.as_secs());
		println!("Average total latency is {} secs per task", average_total_latency.as_secs());
	}

	pub fn all_tasks_completed(&self) -> bool {
		self.tasks.iter().all(|(_, phase)| phase.finish_time.is_some())
	}

	pub fn get_finished_tasks(&self) -> BTreeSet<TaskId> {
		self.tasks
			.iter()
			.filter(|(_, phase)| phase.finish_time.is_some())
			.map(|(id, _)| *id)
			.collect::<BTreeSet<_>>()
	}
}

#[derive(Debug)]
pub struct RecvTaskPhase {
	pub insert_time: Instant,
	pub gmp_in_task: u64,
	pub start_time: Option<Instant>,
	pub finish_time: Option<Instant>,
}

impl Default for RecvTaskPhase {
	fn default() -> Self {
		Self::new()
	}
}

impl RecvTaskPhase {
	pub fn new() -> Self {
		Self {
			insert_time: Instant::now(),
			gmp_in_task: 0,
			start_time: None,
			finish_time: None,
		}
	}

	pub fn start_task(&mut self) {
		if self.start_time.is_none() {
			self.start_time = Some(Instant::now());
		}
	}

	pub fn update_gmp_tasks(&mut self, gmp_tasks: u64) {
		self.gmp_in_task = gmp_tasks;
	}

	pub fn finish_task(&mut self) {
		if self.finish_time.is_none() {
			self.finish_time = Some(Instant::now());
		}
	}

	pub fn is_task_finished(&self) -> bool {
		self.finish_time.is_some()
	}

	pub fn get_start_duration(&self) -> Option<Duration> {
		self.start_time.map(|start| start.duration_since(self.insert_time))
	}

	pub fn get_execution_time(&self) -> Option<Duration> {
		match (self.start_time, self.finish_time) {
			(Some(start_time), Some(finish_time)) => Some(finish_time.duration_since(start_time)),
			_ => None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct TaskPhaseInfo {
	pub insert_time: Instant,
	pub sign_phase_start: Option<Instant>,
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
			insert_time: Instant::now(),
			sign_phase_start: None,
			write_phase_start: None,
			read_phase_start: None,
			finish_time: None,
		}
	}

	pub fn shift_phase(&mut self, phase: TaskPhase) {
		if self.finish_time.is_none() {
			match phase {
				// task start time is task sign phase
				TaskPhase::Sign => self.enter_sign_phase(),
				TaskPhase::Write => self.enter_write_phase(),
				TaskPhase::Read => self.enter_read_phase(),
			}
		}
	}

	pub fn enter_sign_phase(&mut self) {
		if self.sign_phase_start.is_none() {
			self.sign_phase_start = Some(Instant::now());
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

	pub fn unassigned_time(&self) -> Option<Duration> {
		self.sign_phase_start
			.map(|sign_start| sign_start.duration_since(self.insert_time))
	}

	pub fn sign_to_write_duration(&self) -> Option<Duration> {
		match (self.sign_phase_start, self.write_phase_start) {
			(Some(sign_start), Some(write_start)) => Some(write_start.duration_since(sign_start)),
			_ => None,
		}
	}

	pub fn write_to_read_duration(&self) -> Option<Duration> {
		match (self.write_phase_start, self.read_phase_start) {
			(Some(write_start), Some(read_start)) => Some(read_start.duration_since(write_start)),
			_ => None,
		}
	}

	pub fn read_to_finish_duration(&self) -> Option<Duration> {
		match (self.read_phase_start, self.finish_time) {
			(Some(read_start), Some(finish_time)) => Some(finish_time.duration_since(read_start)),
			_ => None,
		}
	}

	pub fn total_execution_duration(&self) -> Option<Duration> {
		self.finish_time.map(|finish_time| finish_time.duration_since(self.insert_time))
	}
}

fn sum_duration(duration: Vec<Duration>) -> Duration {
	duration.iter().fold(Duration::new(0, 0), |acc, &dur| acc + dur)
}

pub async fn test_setup(
	tester: &Tester,
	contract: &Path,
	shard_size: u16,
	threshold: u16,
) -> Result<(EthContractAddress, EthContractAddress, u64)> {
	tester.set_shard_config(shard_size, threshold).await?;
	tester.faucet().await;
	let gmp_contract = tester.setup_gmp(false, None, vec![]).await?;
	let (contract, start_block) = tester
		.deploy(contract, VotingContract::constructorCall { _gateway: gmp_contract.into() })
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
) -> Result<(EthContractAddress, EthContractAddress)> {
	src.faucet().await;
	dest.faucet().await;
	let src_proxy = src.get_proxy_addr().await?;
	let dest_proxy = dest.get_proxy_addr().await?;
	let mut networks = vec![];
	networks.push(Network {
		id: src.network_id(),
		gateway: src_proxy.into_array().into(),
	});
	if src.network_id() != dest.network_id() {
		networks.push(Network {
			id: dest.network_id(),
			gateway: dest_proxy.into_array().into(),
		});
	}
	println!("deploying from source");
	let src_proxy_contract = src.setup_gmp(false, None, networks.clone()).await?;
	// the reason for reverse is to set network info and this way we can compute relative gas price in order
	networks.reverse();
	println!("deploying from destination");
	let dest_proxy_contract = dest.setup_gmp(false, None, networks).await?;

	// deploy testing contract for source chain
	let (src_contract, _) = src
		.deploy(
			contract,
			VotingContract::constructorCall {
				_gateway: src_proxy_contract.into(),
			},
		)
		.await?;

	// deploy testing contract for destination/target chain
	let (dest_contract, _) = dest
		.deploy(
			contract,
			VotingContract::constructorCall {
				_gateway: dest_proxy_contract.into(),
			},
		)
		.await?;

	let src_network = src.network_id();
	let dest_network = dest.network_id();

	// for astar networks we need to deposit funds to voting contract.
	// because frontier considers this an account so when we execute a tx of gmp
	// it thinks account is going empty and throws outoffunds error
	if src_network == 7 || src_network == 6 {
		src.wallet()
			.eth_send_call(
				src_contract,
				VotingContract::depositCall {}.abi_encode(),
				1_000_000,
				None,
				None,
			)
			.await?;
	}

	// registers destination contract in source contract to inform gmp compatibility.
	src.wallet()
		.eth_send_call(
			src_contract,
			VotingContract::registerGmpContractCall {
				_registered: GmpVotingContract {
					dest: dest_contract.into(),
					network: dest_network,
				},
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
			VotingContract::registerGmpContractCall {
				_registered: GmpVotingContract {
					dest: src_contract.into(),
					network: src_network,
				},
			}
			.abi_encode(),
			0,
			None,
			None,
		)
		.await?;

	Ok((src_contract, dest_contract))
}

///
/// Returns gas price of destination chain, in terms of the source chain token
///
/// # Argument
/// `src`: source network_id
/// `dest`: dest network_id
pub fn get_network_ratio(src: NetworkId, dest: NetworkId) -> (u64, u64) {
	if src == dest {
		return (1, 1);
	}
	match (src, dest) {
		(3, 6) => (2, 1),
		(6, 3) => (1, 2),
		_ => (1, 1),
	}
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

	let calls_stream = stream::iter(calls.into_iter());
	let mut chunks = calls_stream.chunks(chunk_size);

	while let Some(chunk) = chunks.next().await {
		let futures = chunk.into_iter().collect::<Vec<_>>();
		let chunk_results: Result<Vec<_>, _> = join_all(futures).await.into_iter().collect();
		results.extend(chunk_results?);
		println!("Waiting gmp send calls: {:?}/{:?}", results.len(), number_of_calls);
	}

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

pub fn rational_to_ufloat(numerator: BigUint, denominator: BigUint) -> Result<u64> {
	let float = UFloat9x56::rational_to_float(numerator, denominator, Rounding::NearestTiesEven)
		.map_err(|e| anyhow::anyhow!("{:?}", e))?;
	Ok(float)
}
