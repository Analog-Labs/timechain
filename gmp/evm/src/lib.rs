use alloy::eips::{BlockId, BlockNumberOrTag};
use alloy::network::{EthereumWallet, TransactionBuilder};
use alloy::primitives::{B256, U256};
use alloy::providers::fillers::{
	BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
};
use alloy::providers::{Provider, ProviderBuilder, RootProvider, WsConnect};
use alloy::rpc::types::{Filter, TransactionRequest};
use alloy::signers::k256::ecdsa::SigningKey;
use alloy::signers::k256::Secp256k1;
use alloy::signers::local::coins_bip39::English;
use alloy::signers::local::{LocalSigner, MnemonicBuilder};
use alloy::sol_types::{SolCall, SolConstructor, SolEvent, SolValue};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use sha3::{Digest, Keccak256};
use sol::IExecutor::{self, IExecutorInstance};
use sol::{u256, TssKey};
use std::ops::Range;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;
use time_primitives::{
	Address32, BatchId, ConnectorParams, Gateway, GatewayMessage, GmpEvent, GmpMessage, Hash,
	IChain, IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route,
	TssPublicKey, TssSignature,
};
use tokio::sync::Mutex;

use crate::sol::CCTP;
use crate::sol::{ProxyContext, ProxyDigest};

type Address20 = alloy::primitives::Address;
type CctpRetryCount = u8;
const MAX_CCTP_RETRY: CctpRetryCount = 3;

pub(crate) mod sol;

fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}

fn t_addr(address: Address20) -> Address32 {
	address.into_word().into()
}

type CProvider = FillProvider<
	JoinFill<
		JoinFill<
			alloy::providers::Identity,
			JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
		>,
		WalletFiller<EthereumWallet>,
	>,
	RootProvider,
>;

#[derive(Clone)]
pub struct Connector {
	network_id: NetworkId,
	rpc: Arc<CProvider>,
	url: String,
	signer: Arc<LocalSigner<SigningKey>>,
	cctp_queue: Arc<Mutex<Vec<CctpRequest>>>,
	// Temporary fix to avoid nonce overlap
	wallet_guard: Arc<Mutex<()>>,
}

#[async_trait]
impl IConnectorBuilder for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		let signer = MnemonicBuilder::<English>::default()
			.phrase(params.mnemonic)
			.index(0)?
			.build()?;

		let ws = WsConnect::new(params.url.clone());
		let provider = Arc::new(ProviderBuilder::new().wallet(signer.clone()).on_ws(ws).await?);

		Ok(Self {
			network_id: params.network_id,
			url: params.url,
			rpc: provider,
			signer: Arc::new(signer),
			cctp_queue: Default::default(),
			wallet_guard: Default::default(),
		})
	}
}

#[async_trait]
impl IChain for Connector {
	/// Formats an address into a string.
	fn format_address(&self, address: Address32) -> String {
		a_addr(address).to_string()
	}
	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address32> {
		Ok(address.parse::<Address20>().map(t_addr)?)
	}
	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		self.network_id
	}
	/// Human readable connector account identifier.
	fn address(&self) -> Address32 {
		t_addr(self.signer.address())
	}
	fn currency(&self) -> (u32, &str) {
		(18, "ETH")
	}
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, _balance: u128) -> Result<()> {
		Err(anyhow!("Faucet not supported"))
	}
	/// Transfers an amount to an account.
	async fn transfer(&self, address: Address32, amount: u128) -> Result<()> {
		let to = a_addr(address);
		let tx = TransactionRequest::default()
			.with_from(self.signer.address())
			.with_to(to)
			.with_value(U256::from(amount));
		let _tx_hash = self.rpc.send_transaction(tx).await?.watch().await?;

		Ok(())
	}
	/// Queries the account balance.
	async fn balance(&self, address: Address32) -> Result<u128> {
		Ok(self.rpc.get_balance(a_addr(address)).await?.try_into()?)
	}
	async fn finalized_block(&self) -> Result<u64> {
		Ok(self
			.rpc
			.get_block(BlockId::finalized())
			.await?
			.ok_or(anyhow!("failed querying finalized block"))?
			.header
			.number)
	}
	/// Stream of finalized block indexes.
	async fn block_stream(&self) -> Result<Pin<Box<dyn Stream<Item = u64> + Send>>> {
		let subscription = self.rpc.subscribe_blocks().await?;
		let stream = subscription.into_stream().map(|b| b.inner.number);

		Ok(stream.boxed() as Pin<Box<dyn Stream<Item = u64> + Send>>)
	}
}

#[async_trait]
impl IConnector for Connector {
	/// Reads gmp messages from the target chain.
	async fn read_events(
		&self,
		gateway: Gateway,
		blocks: Range<u64>,
		cctp_info: Option<(Vec<Address32>, String)>,
	) -> Result<Vec<GmpEvent>> {
		let contract = a_addr(gateway);
		let filter = Filter::new()
			.address(contract)
			.from_block(BlockNumberOrTag::Number(blocks.start))
			// NOTE: rust range is end exclusive, whereas ETH RPC is end inclusive
			.to_block(BlockNumberOrTag::Number(blocks.end - 1));

		let sub = self.rpc.subscribe_logs(&filter).await?;
		let mut stream = sub.into_stream();

		let mut events = vec![];
		while let Some(ref outer_log) = stream.next().await {
			let topics =
				outer_log.topics().iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
			let log = alloy::primitives::Log::new(
				a_addr(gateway),
				topics,
				outer_log.data().data.to_vec().into(),
			)
			.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
			for topic in log.topics() {
				match *topic {
					sol::Gateway::ShardsRegistered::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardsRegistered::decode_log(&log, true)?;
						for key in log.keys.iter() {
							events.push(GmpEvent::ShardRegistered(key.clone().into()));
						}
					},
					sol::Gateway::ShardsUnregistered::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardsUnregistered::decode_log(&log, true)?;
						for key in log.keys.iter() {
							events.push(GmpEvent::ShardUnregistered(key.clone().into()));
						}
						break;
					},
					sol::Gateway::GmpCreated::SIGNATURE_HASH => {
						let log = sol::Gateway::GmpCreated::decode_log(&log, true)?;
						let gmp_message = GmpMessage {
							src_network: self.network_id,
							dest_network: log.destinationNetwork,
							src: log.source.into(),
							dest: t_addr(log.destinationAddress),
							nonce: log.nonce,
							gas_limit: log.executionGasLimit.into(),
							gas_cost: log.gasCost.into(),
							bytes: log.data.data.into(),
						};
						if let Some((ref cctp_contracts, ref url)) = cctp_info {
							if cctp_contracts.contains(&gmp_message.src) {
								let mut cctp_queue = self.cctp_queue.lock().await;
								cctp_queue.push(CctpRequest::new(gmp_message.clone(), url.clone()));
								continue;
							}
						}
						tracing::info!("gmp created: {:?}", hex::encode(gmp_message.message_id()));
						events.push(GmpEvent::MessageReceived(gmp_message));
						break;
					},
					sol::Gateway::GmpExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::GmpExecuted::decode_log(&log, true)?;
						tracing::info!("gmp executed: {:?}", hex::encode(log.id));
						events.push(GmpEvent::MessageExecuted(log.id.into()));
						break;
					},
					sol::Gateway::BatchExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::BatchExecuted::decode_log(&log, true)?;
						events.push(GmpEvent::BatchExecuted {
							batch_id: log.batch,
							tx_hash: outer_log.transaction_hash.map(|hash| hash.into()),
						});
						break;
					},
					_ => {},
				}
			}
		}
		// CCTP calls processing
		let msgs = self.process_cctp_queue().await;
		for msg in msgs {
			events.push(GmpEvent::MessageReceived(msg));
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
		let signature = IExecutor::Signature {
			xCoord: u256(&signer[1..33]),
			e: u256(&sig[..32]),
			s: u256(&sig[32..]),
		};
		// Adding extra overhead for gateway call
		let total_gas = msg.gas().saturating_add(100_000u128);
		let gas_limit: u64 = total_gas.try_into().unwrap_or_else(|_| {
			tracing::error!("Gas {:?} could not be converted to u64", total_gas);
			u64::MAX
		});
		let ops: Vec<IExecutor::GatewayOp> = msg.ops.iter().map(|op| op.clone().into()).collect();
		let message = IExecutor::InboundMessage {
			version: 0,
			batchID: batch,
			ops,
		};
		tracing::info!("submitting batch {batch} with {gas_limit} gas");

		let address = a_addr(gateway);
		let gw = IExecutorInstance::new(address, self.rpc.clone());

		let _pending_tx =
			gw.batchExecute(signature, message).gas(gas_limit).send().await.map_err(|err| {
				tracing::info!("failed to submit batch: {:?}", err);
				err.to_string()
			})?;

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
	) -> Result<(Address32, u64)> {
		let config: DeploymentConfig = serde_json::from_slice(additional_params)?;
		let proxy = extract_bytecode(proxy)?;
		let gateway = extract_bytecode(gateway)?;
		// deploy factory
		let factory_address = a_addr(self.parse_address(&config.factory_address)?).0 .0;
		let factory_deployed_code = self.rpc.get_code_at(factory_address.into()).await?;

		if factory_deployed_code.is_empty() {
			self.deploy_factory_contract(&config).await?;
		}
		// compute proxy address
		let admin = a_addr(self.address());
		let constructor = sol::GatewayProxy::constructorCall { admin };
		let proxy_address =
			compute_create2_address(factory_address, config.deployment_salt, &proxy, constructor)?;
		// check if proxy is deployed
		let proxy_deployed_code = self.rpc.get_code_at(proxy_address.into()).await?;
		if !proxy_deployed_code.is_empty() {
			tracing::debug!("Proxy already deployed, Please upgrade the gateway contract");
			return Ok((t_addr(proxy_address), 0));
		}
		// deploy gateway
		let gateway_address = self.deploy_gateway_contract(&config, proxy_address, gateway).await?;
		// compute proxy arguments
		let (proxy_address, block) = self
			.deploy_proxy_contract(&config, proxy_address, gateway_address, proxy)
			.await?;

		Ok((t_addr(proxy_address), block))
	}

	/// Redeploys the gateway contract.
	async fn redeploy_gateway(
		&self,
		additional_params: &[u8],
		proxy: Address32,
		gateway: &[u8],
	) -> Result<()> {
		let config: DeploymentConfig = serde_json::from_slice(additional_params)?;
		let gateway = extract_bytecode(gateway)?;
		let proxy_address = a_addr(proxy);

		let gateway_addr = self.deploy_gateway_contract(&config, proxy_address, gateway).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: gateway_addr,
		};

		let tx = TransactionRequest::default()
			.with_to(proxy_address)
			.with_chain_id(self.rpc.get_chain_id().await?)
			.with_call(&call);

		self.rpc
			.send_transaction(tx)
			.await?
			.with_timeout(Some(std::time::Duration::from_secs(60)))
			.watch()
			.await?;

		Ok(())
	}
	/// Returns the gateway admin.
	async fn admin(&self, gateway: Address32) -> Result<Address32> {
		// let result = self.evm_view(gateway, sol::Gateway::adminCall {}, None).await?;
		// Ok(t_addr(result._0))
		Err(anyhow!("not implemented yet"))
	}
	/// Sets the gateway admin.
	async fn set_admin(&self, gateway: Address32, admin: Address32) -> Result<()> {
		// let call = sol::Gateway::setAdminCall { admin: a_addr(admin) };
		// self.evm_call(gateway, call, 0, None, None).await?;
		// Ok(())
		Err(anyhow!("not implemented yet"))
	}
	/// Returns the registered shard keys.
	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>> {
		// let result = self.evm_view(gateway, sol::Gateway::shardsCall {}, None).await?;
		// let keys = result._0.into_iter().map(Into::into).collect();
		// Ok(keys)
		Err(anyhow!("not implemented yet"))
	}
	/// Sets the registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address32, keys: &[TssPublicKey]) -> Result<()> {
		// let mut shards = keys.iter().copied().map(Into::into).collect::<Vec<TssKey>>();
		// shards.sort_by(|a, b| a.xCoord.cmp(&b.xCoord));
		// let call = sol::Gateway::setShardsCall { publicKeys: shards };
		// self.evm_call(gateway, call, 0, None, None).await?;
		// Ok(())
		Err(anyhow!("not implemented yet"))
	}
	/// Returns the gateway routing table.
	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>> {
		// let result = self.evm_view(gateway, sol::Gateway::routesCall {}, None).await?;
		// let networks = result._0.into_iter().map(Into::into).collect();
		// Ok(networks)
		Err(anyhow!("not implemented yet"))
	}
	/// Updates an entry in the gateway routing table.
	async fn set_route(&self, gateway: Address32, route: Route) -> Result<()> {
		// let call = sol::Gateway::setRouteCall { info: route.into() };
		// self.evm_call(gateway, call, 0, None, None).await?;
		// Ok(())
		Err(anyhow!("not implemented yet"))
	}
	/// Estimates the message gas limit.
	async fn estimate_message_gas_limit(
		&self,
		contract: Address32,
		src_network: NetworkId,
		src: Address32,
		payload: Vec<u8>,
	) -> Result<u128> {
		// let call = sol::IGmpReceiver::onGmpReceivedCall {
		// 	id: [0; 32].into(),
		// 	network: src_network.into(),
		// 	source: src.into(),
		// 	nonce: 0,
		// 	payload: payload.into(),
		// };
		// let gas_limit = self
		// 	.wallet
		// 	.eth_send_call_estimate_gas(a_addr(contract).into(), call.abi_encode(), 0)
		// 	.await?;
		// Ok(gas_limit)
		Err(anyhow!("not implemented yet"))
	}
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address32,
		dest_network: NetworkId,
		gas_limit: u128,
		payload: Vec<u8>,
	) -> Result<u128> {
		// let msg = sol::GmpMessage {
		// 	source: [0; 32].into(),
		// 	srcNetwork: 0,
		// 	dest: [0; 20].into(),
		// 	destNetwork: 0,
		// 	gasLimit: 0,
		// 	nonce: 0,
		// 	data: payload.into(),
		// };
		// let call = sol::Gateway::estimateMessageCostCall {
		// 	networkid: dest_network,
		// 	// abi_encoded_size returns the size without the 4 byte selector
		// 	messageSize: U256::from(msg.abi_encoded_size() + 4),
		// 	gasLimit: U256::from(gas_limit),
		// };
		// let result = self.evm_view(gateway, call, None).await?;
		// let msg_cost: u128 = result._0.try_into().unwrap();
		// Ok(msg_cost)
		Err(anyhow!("not implemented yet"))
	}

	/// Deploys a test contract.
	async fn deploy_test(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)> {
		// let bytecode = extract_bytecode(tester)?;
		// self.deploy_contract(bytecode, sol::GmpTester::constructorCall { gateway: a_addr(gateway) })
		// 	.await
		Err(anyhow!("not implemented yet"))
	}

	// Sends a message using the test contract.
	async fn send_message(
		&self,
		contract: Address32,
		dest_network: NetworkId,
		dest: Address32,
		gas_limit: u128,
		gas_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		// let msg = sol::GmpMessage {
		// 	srcNetwork: self.network_id,
		// 	source: contract.into(),
		// 	destNetwork: dest_network,
		// 	dest: a_addr(dest),
		// 	nonce: 0,
		// 	gasLimit: gas_limit as _,
		// 	data: payload.into(),
		// };
		// tracing::debug!("Sending GMP message: {:#?}", &msg);
		// let call = sol::GmpTester::sendMessageCall { msg };
		// let result = self.evm_call(contract, call, gas_cost, None, None).await?;
		// let id: MessageId = *result.0._0;
		// Ok(id)
		Err(anyhow!("not implemented yet"))
	}

	/// Receives messages from test contract.
	async fn recv_messages(
		&self,
		contract: Address32,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		// let contract: [u8; 20] = a_addr(contract).0.into();
		// let logs = self
		// 	.wallet
		// 	.query(GetLogs {
		// 		contracts: vec![contract.into()],
		// 		topics: vec![],
		// 		block: FilterBlockOption::Range {
		// 			from_block: Some(blocks.start.into()),
		// 			to_block: Some(blocks.end.into()),
		// 		},
		// 	})
		// 	.await?;
		// let mut msgs = vec![];
		// for log in logs {
		// 	let topics = log.topics.iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
		// 	let log =
		// 		alloy::primitives::Log::new(contract.into(), topics, log.data.0.to_vec().into())
		// 			.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
		// 	for topic in log.topics() {
		// 		let sol::GmpTester::MessageReceived::SIGNATURE_HASH = *topic else {
		// 			continue;
		// 		};
		// 		let log = sol::GmpTester::MessageReceived::decode_log(&log, true)?;
		// 		let msg: GmpMessage = log.msg.clone().into();
		// 		msgs.push(msg);
		// 	}
		// }
		// Ok(msgs)
		Err(anyhow!("not implemented yet"))
	}

	/// Get EIP1559 `max_fee_per_gas` estimate for a chain.
	async fn max_fee_per_gas(&self) -> Result<u128> {
		// let fee_estimator = if self.wallet.config().blockchain == "polygon" {
		// 	self.backend.estimate_eip1559_fees::<PolygonFeeEstimatorConfig>().await?
		// } else {
		// 	self.backend.estimate_eip1559_fees::<DefaultFeeEstimatorConfig>().await?
		// };
		// Ok(u128::try_from(fee_estimator.0)
		// 	.map_err(|_| anyhow::anyhow!("Failed to convert value from U256 to u128"))?)
		Err(anyhow!("not implemented yet"))
	}

	/// Returns gas limit of latest block.
	async fn block_gas_limit(&self) -> Result<u64> {
		// let block = self
		// 	.backend
		// 	.block(AtBlock::Latest)
		// 	.await?
		// 	.with_context(|| "Cannot find latest block")?;
		// Ok(block.header.gas_limit)
		Err(anyhow!("not implemented yet"))
	}

	/// Withdraw gateway funds.
	async fn withdraw_funds(
		&self,
		gateway: Address32,
		amount: u128,
		receipient: Address32,
	) -> Result<()> {
		// let call = sol::Gateway::withdrawCall {
		// 	amount: U256::from(amount),
		// 	recipient: a_addr(receipient),
		// 	data: vec![].into(),
		// };
		// self.evm_call(gateway, call, 0, None, None).await?;
		// Ok(())
		Err(anyhow!("not implemented yet"))
	}
	/// Debug a transaction.
	async fn debug_transaction(&self, hash: Hash) -> Result<String> {
		let analog_gmp_dir =
			std::env::var("ANALOG_GMP_DIR").context("failed to find ANALOG_GMP_DIR")?;
		let output = Command::new("cast")
			.current_dir(analog_gmp_dir)
			.arg("run")
			.arg("--rpc-url")
			.arg(&self.url)
			.arg("--with-local-artifacts")
			.arg(hex::encode(hash))
			.output()
			.context("failed to run cast")?;
		if !output.status.success() {
			let err = std::str::from_utf8(&output.stderr).ok().unwrap_or_default();
			anyhow::bail!("cast exited with {}: {err}", output.status);
		}
		let stdout = std::str::from_utf8(&output.stdout)?;
		Ok(stdout.into())
	}
	/// Dump anvil chain state
	async fn dump_state(&self) -> Result<String> {
		let body = serde_json::json!({
			"id": 0,
			"jsonrpc": "2.0",
			"method": "anvil_dumpState",
			"params": []
		});
		let json: serde_json::Value = reqwest::Client::new()
			.post(self.url.replace("ws", "http"))
			.json(&body)
			.send()
			.await?
			.json()
			.await?;

		json["result"]
			.as_str()
			.map(|s| s.to_owned())
			.ok_or(anyhow!("invalid rpc response"))
	}
	/// Load anvil chain state
	async fn load_state(&self, state: String) -> Result<()> {
		let body = serde_json::json!({
			"id": 0,
			"jsonrpc": "2.0",
			"method": "anvil_loadState",
			"params": [ state ]
		});

		let json: serde_json::Value = reqwest::Client::new()
			.post(self.url.replace("ws", "http"))
			.json(&body)
			.send()
			.await?
			.json()
			.await?;

		if !json["error"].is_null() {
			return Err(anyhow!("{}", json["error"].to_string()));
		}

		Ok(())
	}
}

impl Connector {
	/// init_code == contract_bytecode + contractor_code
	async fn deploy_contract_with_factory(
		&self,
		config: &DeploymentConfig,
		call: Vec<u8>,
	) -> Result<(Address20, u64)> {
		let factory_address = a_addr(self.parse_address(&config.factory_address)?);

		let tx = TransactionRequest::default()
			.with_to(factory_address)
			.with_chain_id(self.rpc.get_chain_id().await?)
			// TODO why magic value
			.with_gas_limit(20_000_000)
			.with_input(call);

		let guard = self.wallet_guard.lock().await;
		let pending_tx = self.rpc.send_transaction(tx).await?;
		drop(guard);
		let tx_hash = pending_tx.tx_hash().clone();
		tracing::debug!("deployment tx: {tx_hash}");

		let receipt = pending_tx.get_receipt().await?;
		tracing::debug!("deployment tx receipt: {receipt:?}");

		let log = receipt
			.logs()
			.iter()
			.find(|log| log.address() == factory_address)
			.with_context(|| format!("tx {tx_hash} logs not found"))?;

		let topic =
			log.topics().first().with_context(|| format!("tx {tx_hash} topic not found"))?;

		let contract_address = Address20::from_slice(&topic[12..]);
		Ok((contract_address, receipt.block_number.unwrap()))
	}

	// TODO this needs refactoring: why deployer and contract bytecode are hard-coded?
	async fn deploy_factory_contract(&self, config: &DeploymentConfig) -> Result<()> {
		let deployer_address = self.parse_address(&config.factory_deployer)?;
		// Step1: fund 0x908064dE91a32edaC91393FEc3308E6624b85941
		self.transfer(deployer_address, config.required_balance).await?;
		//Step2: load transaction from config
		let encoded_tx = hex::decode(config.raw_tx.strip_prefix("0x").unwrap_or(&config.raw_tx))?;
		//Step3: send eth_rawTransaction
		let tx_hash = self
			.rpc
			.send_raw_transaction(&encoded_tx)
			.await?
			.with_timeout(Some(std::time::Duration::from_secs(60)))
			.watch()
			.await?;
		tracing::info!("factory deployed with tx {:?}", tx_hash);

		Ok(())
	}

	async fn deploy_gateway_contract(
		&self,
		config: &DeploymentConfig,
		proxy: Address20,
		mut bytecode: Vec<u8>,
	) -> Result<Address20> {
		let constructor = sol::Gateway::constructorCall {
			network: self.network_id,
			proxy,
		};
		bytecode.extend(constructor.abi_encode());
		let call = sol::IUniversalFactory::create2_0Call {
			salt: config.deployment_salt.into(),
			creationCode: bytecode.into(),
		}
		.abi_encode();
		let (gateway_address, _) = self.deploy_contract_with_factory(config, call).await?;
		tracing::info!("gateway deployed at {}", gateway_address);

		Ok(gateway_address)
	}

	async fn deploy_proxy_contract(
		&self,
		config: &DeploymentConfig,
		proxy_addr: Address20,
		gateway_address: Address20,
		mut bytecode: Vec<u8>,
	) -> Result<(Address20, u64)> {
		// // constructor params
		// let admin = a_addr(self.address());
		// let constructor = sol::GatewayProxy::constructorCall { admin };
		// bytecode.extend(constructor.abi_encode());

		// // computing signature for security purpose
		// let digest = ProxyDigest {
		// 	proxy: proxy_addr,
		// 	implementation: gateway_address,
		// }
		// .abi_encode();
		// let payload: [u8; 32] = Keccak256::digest(digest).into();
		// let sig = self.wallet.sign_prehashed(&payload)?.to_bytes();
		// debug_assert!(sig.len() == 65);
		// let r: [u8; 32] = sig[0..32].try_into()?;
		// let s: [u8; 32] = sig[32..64].try_into()?;
		// let v = sig[64];
		// let arguments = ProxyContext {
		// 	// Ethereum verification uses 27,28 instead of 0,1 for recovery id
		// 	v: v + 27,
		// 	r: r.into(),
		// 	s: s.into(),
		// 	implementation: gateway_address,
		// }
		// .abi_encode();

		// let initializer = sol::Gateway::initializeCall {
		// 	admin,
		// 	keys: vec![],
		// 	networks: vec![],
		// }
		// .abi_encode();

		// // Proxy creation
		// let call = sol::IUniversalFactory::create2_1Call {
		// 	salt: config.deployment_salt.into(),
		// 	creationCode: bytecode.into(),
		// 	arguments: arguments.into(),
		// 	callback: initializer.into(),
		// }
		// .abi_encode();

		// let (proxy_address, block) = self.deploy_contract_with_factory(config, call).await?;

		// if proxy_address != proxy_addr {
		// 	anyhow::bail!(
		// 		"Unable to compute proxy address: expected: {:?}, got {:?}",
		// 		proxy_addr,
		// 		proxy_address
		// 	);
		// }
		// tracing::info!("proxy deployed at {}", proxy_address);
		// Ok((proxy_address, block))
		Err(anyhow!("not implemented yet"))
	}

	async fn process_cctp_msg(&self, request: &mut CctpRequest) -> Result<(), CctpError> {
		let payload = request.msg.bytes.clone();
		let mut cctp_payload =
			CCTP::abi_decode(&payload, false).map_err(|_| CctpError::InvalidPayload)?;
		if cctp_payload.get_version().map_err(|_| CctpError::InvalidPayload)? != 0 {
			return Err(CctpError::InvalidVersion);
		}
		let burn_message: Vec<u8> = cctp_payload.message.clone().into();
		let burn_hash: [u8; 32] = sha3::Keccak256::digest(&burn_message).into();
		let attestation_response = self.get_cctp_attestation(burn_hash, &request.url).await?;
		let signature =
			attestation_response.attestation.clone().ok_or(CctpError::AttestationResponse)?;
		let signature = signature.strip_prefix("0x").unwrap_or(&signature);
		let attestation = hex::decode(signature).map_err(|_| CctpError::InvalidSignature)?;
		cctp_payload.attestation = attestation.into();
		request.msg.bytes = cctp_payload.abi_encode();
		Ok(())
	}

	async fn get_cctp_attestation(
		&self,
		burn_hash: [u8; 32],
		uri: &str,
	) -> Result<AttestationResponse, CctpError> {
		let uri = uri.trim_end_matches('/');
		let url = format!("{}/0x{}", uri, hex::encode(burn_hash));
		let client = Client::new();
		let response = client
			.get(&url)
			.send()
			.await
			.map_err(|e| CctpError::InvalidResponse(e.to_string()))?
			.error_for_status()
			.map_err(|e| CctpError::InvalidResponse(e.to_string()))?;
		let attestation_response: AttestationResponse =
			response.json().await.map_err(|e| CctpError::InvalidResponse(e.to_string()))?;
		if attestation_response.status == "complete" {
			return Ok(attestation_response);
		}
		Err(CctpError::AttestationPending)
	}

	async fn process_cctp_queue(&self) -> Vec<GmpMessage> {
		let mut queue = self.cctp_queue.lock().await;
		if queue.is_empty() {
			return vec![];
		}

		let mut attested_msgs = vec![];

		let msgs = std::mem::take(&mut *queue);
		for mut request in msgs {
			match self.process_cctp_msg(&mut request).await {
				Ok(()) => attested_msgs.push(request.msg),
				Err(CctpError::AttestationPending) => {
					request.retry_count += 1;
					if request.retry_count >= MAX_CCTP_RETRY {
						tracing::info!("Dropping Cctp message due to count: {:?}", request);
					} else {
						tracing::info!("Attestation is pending for msg: {:?}", request.msg);
						queue.push(request);
					}
				},
				Err(error) => {
					tracing::error!(
						"Failed to process cctp message: {:?}: {:?}",
						request.msg,
						error
					);
				},
			}
		}

		if !queue.is_empty() {
			tracing::info!("{} Cctp messages have pending attestations.", queue.len());
		}
		attested_msgs
	}
}

fn compute_create2_address(
	factory_address: [u8; 20],
	salt: [u8; 32],
	bytecode: &[u8],
	constructor: impl SolConstructor,
) -> Result<Address20> {
	// solidity
	// bytes32 create2hash = keccak256(abi.encodePacked(uint8(0xff), address(factory), salt, initcodeHash));
	// return address(uint160(uint256(create2hash)));
	let mut hasher = Keccak256::new();
	hasher.update(bytecode);
	hasher.update(constructor.abi_encode());
	let init_code_hash = hasher.finalize();

	let mut hasher = Keccak256::new();
	hasher.update([0xff]);
	hasher.update(factory_address);
	hasher.update(salt);
	hasher.update(init_code_hash);
	let proxy_hashed = hasher.finalize();

	Ok(Address20::from_slice(&proxy_hashed[12..]))
}

fn extract_bytecode(json_abi: &[u8]) -> Result<Vec<u8>> {
	let contract_abi: Contract = serde_json::from_slice(json_abi)?;
	hex::decode(contract_abi.bytecode.object.replace("0x", ""))
		.with_context(|| "Failed to get contract bytecode")
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeploymentConfig {
	pub factory_deployer: String,
	pub required_balance: u128,
	pub raw_tx: String,
	pub factory_address: String,
	pub deployment_salt: [u8; 32],
}

#[derive(Clone, Debug, Deserialize)]
pub struct CctpRequest {
	msg: GmpMessage,
	url: String,
	retry_count: CctpRetryCount,
}

impl CctpRequest {
	fn new(msg: GmpMessage, url: String) -> Self {
		Self { msg, url, retry_count: 0 }
	}
}

#[derive(Deserialize)]
struct Contract {
	bytecode: Bytecode,
}

#[derive(Deserialize)]
struct Bytecode {
	object: String,
}

#[derive(Deserialize, Debug)]
struct AttestationResponse {
	status: String,
	attestation: Option<String>,
}

#[derive(Error, Debug)]
enum CctpError {
	#[error("Attestation is pending.")]
	AttestationPending,
	#[error("Failed to get attestation from response.")]
	AttestationResponse,
	#[error("Invalid payload.")]
	InvalidPayload,
	#[error("Invalid response {0}.")]
	InvalidResponse(String),
	#[error("Invalid signature.")]
	InvalidSignature,
	#[error("Cctp version is invalid.")]
	InvalidVersion,
}
