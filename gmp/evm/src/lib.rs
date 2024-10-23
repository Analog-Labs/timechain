use alloy_primitives::{B256, U256};
use alloy_sol_types::{SolCall, SolConstructor, SolEvent};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::Stream;
use rosetta_client::{
	query::GetLogs, types::AccountIdentifier, AtBlock, CallResult, FilterBlockOption,
	GetTransactionCount, SubmitResult, TransactionReceipt, Wallet,
};
use rosetta_ethereum_backend::{jsonrpsee::Adapter, EthereumRpc};
use rosetta_server::ws::{default_client, DefaultClient};
use rosetta_server_ethereum::utils::{
	DefaultFeeEstimatorConfig, EthereumRpcExt, PolygonFeeEstimatorConfig,
};
use serde::Deserialize;
use sha3::Keccak256;
use std::io::BufReader;
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
use std::{fs::File, ops::Range};
use time_primitives::{
	Address, BatchId, ConnectorParams, Gateway, GatewayMessage, GmpEvent, GmpMessage, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, NetworkId, Route, TssPublicKey, TssSignature,
};

pub(crate) mod sol;

fn a_addr(address: Address) -> alloy_primitives::Address {
	let address: [u8; 20] = address[..20].try_into().unwrap();
	alloy_primitives::Address::from(address)
}

fn t_addr(address: alloy_primitives::Address) -> Address {
	let mut addr = [0; 32];
	addr[..20].copy_from_slice(&address.0[..]);
	addr
}

#[derive(Clone)]
pub struct Connector {
	network_id: NetworkId,
	wallet: Arc<Wallet>,
	backend: Adapter<DefaultClient>,
}

impl Connector {
	async fn nonce(&self, address: Address) -> Result<u64> {
		let address: [u8; 20] = address[..20].try_into().unwrap();
		self.wallet
			.query(GetTransactionCount {
				address: address.into(),
				block: AtBlock::Latest,
			})
			.await
	}

	async fn evm_call<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		amount: u128,
		nonce: Option<u64>,
	) -> Result<(T::Return, TransactionReceipt, [u8; 32])> {
		let contract: [u8; 20] = contract[..20].try_into().unwrap();
		let result = self
			.wallet
			.eth_send_call(contract, call.abi_encode(), amount, nonce, None)
			.await?;
		let SubmitResult::Executed {
			result: CallResult::Success(result),
			receipt,
			tx_hash,
		} = result
		else {
			anyhow::bail!("{:?}", result)
		};
		Ok((T::abi_decode_returns(&result, true)?, receipt, tx_hash.into()))
	}

	async fn evm_view<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		block: Option<u64>,
	) -> Result<T::Return> {
		let contract: [u8; 20] = contract[..20].try_into().unwrap();
		let block: AtBlock = if let Some(block) = block { block.into() } else { AtBlock::Latest };
		let result = self.wallet.eth_view_call(contract, call.abi_encode(), block).await?;
		let CallResult::Success(result) = result else { anyhow::bail!("{:?}", result) };
		Ok(T::abi_decode_returns(&result, true)?)
	}

	async fn deploy_contract(
		&self,
		abi: &[u8],
		constructor: impl SolConstructor,
	) -> Result<(Address, u64)> {
		let mut contract = get_contract_from_slice(abi)?;
		contract.extend(constructor.abi_encode());
		let tx_hash = self.wallet.eth_deploy_contract(contract).await?.tx_hash().0;
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash).await?.unwrap();
		let address = tx_receipt.contract_address.unwrap();
		let block_number = tx_receipt.block_number.unwrap();
		Ok((t_addr(address.0.into()), block_number))
	}

	///
	/// init_code == contract_bytecode + contractor_code
	async fn deploy_contract_with_factory(
		&self,
		factory_address: [u8; 20],
		init_code: &[u8],
	) -> Result<(Address, u64)> {
		let tx_hash = self
			.wallet
			.eth_send_call(factory_address, init_code.to_vec(), 0, None, None)
			.await?
			.tx_hash();
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash.into()).await?.unwrap();
		println!("tx_receipt: {:?}", tx_receipt);
		// let address = tx_receipt.contract_address.unwrap();
		// let block_number = tx_receipt.block_number.unwrap();
		// Ok((t_addr(address.0.into()), block_number))
		todo!()
	}

	async fn compute_proxy_address(
		&self,
		factory_address: [u8; 20],
		salt: [u8; 32],
		contract_bytes: Vec<u8>,
	) -> Result<[u8; 20]> {
		use sha3::Digest;
		// let address = keccak256(0xff + 0x0000000000001C4Bf962dF86e38F0c10c7972C6E + 256bit salt + keccak256(contract_initcode))[12..32];

		let mut hasher = sha3::Keccak256::new();
		hasher.update(contract_bytes);
		let contract_code_hash = hasher.finalize();

		let mut hasher = Keccak256::new();
		hasher.update([0xff]);
		hasher.update(&factory_address);
		hasher.update(salt);
		hasher.update(contract_code_hash);

		let final_hash = hasher.finalize();
		let mut final_address = [0u8; 20];
		final_address.copy_from_slice(&final_hash[12..32]);

		Ok(final_address)
	}

	async fn deploy_factory(&self, config: &DeploymentConfig) -> Result<()> {
		let deployer_address = self.parse_address(&config.deployer)?;

		//Step1: fund 0x908064dE91a32edaC91393FEc3308E6624b85941
		self.faucet().await?;
		self.transfer(deployer_address, config.required_balance).await?;

		//Step2: load transaction from config
		let tx = hex::decode(config.raw_tx.strip_prefix("0x").unwrap_or(&config.raw_tx))?;

		//Step3: send eth_rawTransaction
		let tx_hash = self.eth_backend.send_raw_transaction(tx.into()).await?;

		tracing::info!("Deployed factory with hash: {:?}", tx_hash);
		Ok(())
	}
}

#[async_trait]
impl IConnectorBuilder for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		println!("{:?}", params);
		let wallet = Arc::new(
			Wallet::new(
				params.blockchain.parse()?,
				&params.network,
				&params.url,
				&params.mnemonic,
				None,
			)
			.await?,
		);
		let client = default_client(&params.url, None)
			.await
			.with_context(|| "Cannot get ws client for url: {url}")?;
		let adapter = Adapter(client);
		Ok(Self {
			network_id: params.network_id,
			wallet,
			backend: adapter,
		})
	}
}

#[async_trait]
impl IChain for Connector {
	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String {
		a_addr(address).to_string()
	}
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address> {
		let address: alloy_primitives::Address = address.parse()?;
		Ok(t_addr(address))
	}
	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		self.network_id
	}
	/// Human readable connector account identifier.
	fn address(&self) -> Address {
		self.parse_address(&self.wallet.account().address).unwrap()
	}
	fn currency(&self) -> (u32, &str) {
		(18, "ETH")
	}
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self) -> Result<()> {
		let balance = match self.network_id() {
			6 => 10u128.pow(25), // astar
			3 => 10u128.pow(29), // ethereum
			network_id => {
				tracing::info!("network {network_id} doesn't support faucet");
				return Ok(());
			},
		};
		self.wallet.faucet(balance, None).await?;
		Ok(())
	}
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		let address = self.format_address(address);
		self.wallet
			.transfer(&AccountIdentifier::new(address), amount, None, None)
			.await?;
		Ok(())
	}
	/// Queries the account balance.
	async fn balance(&self, address: Address) -> Result<u128> {
		self.wallet.balance(a_addr(address).to_string()).await
	}
	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send>> {
		self.wallet.block_stream()
	}
}

#[async_trait]
impl IConnector for Connector {
	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Gateway, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
		let contract: [u8; 20] = a_addr(gateway).0.into();
		let logs = self
			.wallet
			.query(GetLogs {
				contracts: vec![contract.into()],
				topics: vec![
					sol::Gateway::ShardRegistered::SIGNATURE_HASH.0.into(),
					sol::Gateway::ShardUnregistered::SIGNATURE_HASH.0.into(),
					sol::Gateway::MessageReceived::SIGNATURE_HASH.0.into(),
					sol::Gateway::MessageExecuted::SIGNATURE_HASH.0.into(),
					sol::Gateway::BatchExecuted::SIGNATURE_HASH.0.into(),
				],
				block: FilterBlockOption::Range {
					from_block: Some(blocks.start.into()),
					to_block: Some(blocks.end.into()),
				},
			})
			.await?;
		let mut events = vec![];
		for log in logs {
			let topics = log.topics.iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
			let log =
				alloy_primitives::Log::new(a_addr(gateway), topics, log.data.0.to_vec().into())
					.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
			for topic in log.topics() {
				match *topic {
					sol::Gateway::ShardRegistered::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardRegistered::decode_log(&log, true)?;
						events.push(GmpEvent::ShardRegistered(log.key.clone().into()));
						break;
					},
					sol::Gateway::ShardUnregistered::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardUnregistered::decode_log(&log, true)?;
						events.push(GmpEvent::ShardUnregistered(log.key.clone().into()));
						break;
					},
					sol::Gateway::MessageReceived::SIGNATURE_HASH => {
						let log = sol::Gateway::MessageReceived::decode_log(&log, true)?;
						events.push(GmpEvent::MessageReceived(
							log.msg.clone().outbound(self.network_id),
						));
						break;
					},
					sol::Gateway::MessageExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::MessageExecuted::decode_log(&log, true)?;
						events.push(GmpEvent::MessageExecuted(log.id.into()));
						break;
					},
					sol::Gateway::BatchExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::BatchExecuted::decode_log(&log, true)?;
						events.push(GmpEvent::BatchExecuted(log.batch));
						break;
					},
					_ => {},
				}
			}
		}
		Ok(events)
	}
	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		gateway: Gateway,
		batch: BatchId,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		let call = sol::Gateway::executeCall {
			signature: sig.into(),
			xCoord: sol::TssPublicKey::from(signer).xCoord,
			message: msg.encode(batch).into(),
		};
		self.evm_call(gateway, call, 0, None).await.map_err(|err| err.to_string())?;
		Ok(())
	}
}

#[async_trait]
impl IConnectorAdmin for Connector {
	/// Deploys the gateway contract.
	async fn deploy_gateway(
		&self,
		additional_params: &[u8],
		proxy: &[u8],
		gateway: &[u8],
	) -> Result<(Address, u64)> {
		// load additional config
		let additional_config_path = String::from_utf8(additional_params.to_vec())?;
		let file = File::open(additional_config_path)?;
		let reader = BufReader::new(file);

		// check if uf already deployed
		let config: DeploymentConfig = serde_json::from_reader(reader)?;
		let factory_address = a_addr(self.parse_address(&config.factory_address)?).0 .0;
		let is_factory_deployed = self
			.eth_backend
			.get_code(factory_address.into(), rosetta_ethereum_types::AtBlock::Latest)
			.await?;

		if is_factory_deployed.is_empty() {
			println!("Factory is not deployed");
			self.deploy_factory(&config).await?;
		} else {
			println!("Factory is deployed: {:?}", is_factory_deployed.len());
		}

		// TODO
		let mut proxy_bytecode = get_contract_from_slice(proxy)?;

		//proxy constructor
		let proxy_constructor = sol::GatewayProxy::constructorCall {
			implementation: factory_address.into(),
		};

		let proxy_bytecode =
			extend_bytes_with_constructor(proxy_bytecode.clone(), proxy_constructor);

		// deploy proxy using universal factory
		let computed_proxy_address = self
			.compute_proxy_address(factory_address, [0; 32], proxy_bytecode.clone())
			.await?;
		println!("computed_proxy_address: {:?}", hex::encode(computed_proxy_address));

		let result = self.deploy_contract_with_factory(factory_address, &proxy_bytecode).await?;
		println!("result of deployment: {:?}", result);

		// let gateway_bytecode = get_contract_from_slice(proxy)?;

		// let computed_gateway_address =
		// 	self.compute_proxy_address(factory_address, [0; 32], proxy.to_vec()).await?;

		// let gateway_contstructor = sol::Gateway::constructorCall {
		// 	networkId: self.network_id,
		// 	proxy: a_addr([0u8; 32]),
		// };

		// deploy gateway
		// let admin = self.address();

		// get deployer's nonce
		// let nonce = self.nonce(admin).await?;

		// compute the proxy address
		// let proxy_addr = a_addr(admin).create(nonce + 1);
		// let proxy_addr = t_addr(proxy_addr);

		// deploy gateway
		// let (gateway_addr, _) = self.deploy_contract(gateway, call).await?;

		// // deploy the proxy contract
		// let call = sol::GatewayProxy::constructorCall {
		// 	implementation: a_addr(gateway_addr),
		// };
		// let (actual_addr, block_number) = self.deploy_contract(proxy, call).await?;

		// // Check if the proxy address match the expect address
		// if proxy_addr != actual_addr {
		// 	anyhow::bail!("Proxy address mismatch, expect {proxy_addr:?}, got {actual_addr:?}");
		// }
		let actual_addr = todo!();
		let block_number = todo!();
		Ok((actual_addr, block_number))
	}
	/// Redeploys the gateway contract.
	async fn redeploy_gateway(&self, proxy: Address, gateway: &[u8]) -> Result<()> {
		let call = sol::Gateway::constructorCall {
			networkId: self.network_id,
			proxy: a_addr(proxy),
		};
		let (gateway_addr, _) = self.deploy_contract(gateway, call).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: a_addr(gateway_addr),
		};
		self.evm_call(proxy, call, 0, None).await?;
		Ok(())
	}
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address) -> Result<Address> {
		let result = self.evm_view(gateway, sol::Gateway::adminCall {}, None).await?;
		Ok(t_addr(result._0))
	}
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address, admin: Address) -> Result<()> {
		let call = sol::Gateway::setAdminCall { admin: a_addr(admin) };
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>> {
		let result = self.evm_view(gateway, sol::Gateway::shardsCall {}, None).await?;
		let keys = result._0.into_iter().map(Into::into).collect();
		Ok(keys)
	}
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()> {
		let shards = keys.iter().copied().map(Into::into).collect::<Vec<_>>();
		let call = sol::Gateway::setShardsCall { shards };
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address) -> Result<Vec<Route>> {
		let result = self.evm_view(gateway, sol::Gateway::routesCall {}, None).await?;
		let networks = result._0.into_iter().map(Into::into).collect();
		Ok(networks)
	}
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address, route: Route) -> Result<()> {
		let call = sol::Gateway::setRouteCall { route: route.into() };
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address,
		dest: NetworkId,
		msg_size: usize,
	) -> Result<u128> {
		let msg_size = U256::from_str_radix(&msg_size.to_string(), 16).unwrap();
		let call = sol::Gateway::estimateMessageCostCall {
			networkid: dest,
			messageSize: msg_size,
			gasLimit: U256::from(100_000),
		};
		let result = self.evm_view(gateway, call, None).await?;
		let msg_cost: u128 = result._0.try_into().unwrap();
		Ok(msg_cost)
	}
	/// Deploys a test contract.
	async fn deploy_test(&self, gateway: Address, tester: &[u8]) -> Result<(Address, u64)> {
		self.deploy_contract(tester, sol::GmpTester::constructorCall { gateway: a_addr(gateway) })
			.await
	}
	/// Sends a message using the test contract.
	async fn send_message(&self, contract: Address, msg: GmpMessage) -> Result<()> {
		let call = sol::GmpTester::sendMessageCall {
			msg: sol::GmpMessage::from_outbound(msg),
		};
		self.evm_call(contract, call, 0, None).await?;
		Ok(())
	}
	/// Receives messages from test contract.
	async fn recv_messages(
		&self,
		contract: Address,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		let contract: [u8; 20] = a_addr(contract).0.into();
		let logs = self
			.wallet
			.query(GetLogs {
				contracts: vec![contract.into()],
				topics: vec![sol::GmpTester::MessageReceived::SIGNATURE_HASH.0.into()],
				block: FilterBlockOption::Range {
					from_block: Some(blocks.start.into()),
					to_block: Some(blocks.end.into()),
				},
			})
			.await?;
		let mut msgs = vec![];
		for log in logs {
			let topics = log.topics.iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
			let log =
				alloy_primitives::Log::new(contract.into(), topics, log.data.0.to_vec().into())
					.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
			let log = sol::GmpTester::MessageReceived::decode_log(&log, true)?;
			msgs.push(log.msg.clone().inbound(self.network_id));
		}
		Ok(msgs)
	}
	/// Calculate transaction base fee for a chain.
	async fn transaction_base_fee(&self) -> Result<u128> {
		let fee_estimator = if self.wallet.config().blockchain == "polygon" {
			self.backend.estimate_eip1559_fees::<PolygonFeeEstimatorConfig>().await?
		} else {
			self.backend.estimate_eip1559_fees::<DefaultFeeEstimatorConfig>().await?
		};
		Ok(u128::try_from(fee_estimator.0)
			.map_err(|_| anyhow::anyhow!("Failed to convert value from U256 to u128"))?)
	}

	/// Returns gas limit of latest block.
	async fn block_gas_limit(&self) -> Result<u64> {
		let block = self
			.backend
			.block(AtBlock::Latest)
			.await?
			.with_context(|| "Cannot find latest block")?;
		Ok(block.header.gas_limit)
	}
	/// Withdraw gateway funds.
	async fn withdraw_funds(
		&self,
		gateway: Address,
		amount: u128,
		receipient: Address,
	) -> Result<()> {
		let call = sol::Gateway::withdrawCall {
			amount: U256::from(amount),
			recipient: a_addr(receipient),
			data: vec![].into(),
		};
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}
}

fn get_contract_from_slice(slice: &[u8]) -> Result<Vec<u8>> {
	let contract_abi: Contract = serde_json::from_slice(slice)?;
	hex::decode(contract_abi.bytecode.object.replace("0x", ""))
		.with_context(|| "Failed to get contract bytecode")
}

fn extend_bytes_with_constructor(bytecode: Vec<u8>, constructor: impl SolConstructor) -> Vec<u8> {
	let mut bytecode = bytecode.clone();
	bytecode.extend(constructor.abi_encode());
	bytecode
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeploymentConfig {
	pub deployer: String,
	pub required_balance: u128,
	pub raw_tx: String,
	pub factory_address: String,
}

#[derive(Deserialize)]
struct Contract {
	bytecode: Bytecode,
}

#[derive(Deserialize)]
struct Bytecode {
	object: String,
}
