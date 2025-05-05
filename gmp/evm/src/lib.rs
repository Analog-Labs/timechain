use alloy::{
	eips::{BlockId, BlockNumberOrTag},
	network::{
		AnyHeader, AnyNetwork, AnyReceiptEnvelope, EthereumWallet, ReceiptResponse,
		TransactionBuilder,
	},
	primitives::{B256, U256},
	providers::{
		fillers::{
			BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
			WalletFiller,
		},
		utils::Eip1559Estimator,
		Provider, ProviderBuilder, RootProvider, WsConnect,
	},
	rpc::types::{Filter, Header, Log, TransactionReceipt, TransactionRequest},
	serde::WithOtherFields,
	signers::{
		k256::ecdsa::SigningKey,
		local::{coins_bip39::English, LocalSigner, MnemonicBuilder},
		SignerSync,
	},
	sol_types::{SolCall, SolConstructor, SolEvent, SolValue},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use blocks::FinalizedBlockStream;
use custom::BEP226;
use dict::Currency;
use futures::{Stream, StreamExt};
use serde::Deserialize;
use sha3::{Digest, Keccak256};
use sol::{
	u256,
	IExecutor::{self, IExecutorInstance},
	TssKey,
};
use std::{ops::Range, pin::Pin, process::Command, sync::Arc, time::Duration};
use time_primitives::{
	Address32, BatchId, ConnectorParams, GatewayMessage, GmpEvent, GmpMessage, Hash, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, TssPublicKey,
	TssSignature,
};
use tokio::sync::Mutex;

use crate::cctp::CctpHandler;
use crate::sol::{ProxyContext, ProxyDigest};

type Address20 = alloy::primitives::Address;

pub(crate) mod blocks;
pub(crate) mod cctp;
pub(crate) mod custom;
pub(crate) mod dict;
pub(crate) mod sol;

const DEFAULT_TX_TIMEOUT: u64 = 60;

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
	RootProvider<AnyNetwork>,
	AnyNetwork,
>;

#[derive(Clone)]
pub struct Connector {
	network_id: NetworkId,
	rpc: Arc<CProvider>,
	url: String,
	signer: Arc<LocalSigner<SigningKey>>,
	cctp: Arc<CctpHandler>,
	chain_id: u64,
	currency: Currency,
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
		let ws = WsConnect::new(params.url.clone())
			.with_max_retries(1200)
			.with_retry_interval(Duration::from_secs(3));

		let provider = Arc::new(
			ProviderBuilder::new()
				.network::<AnyNetwork>()
				.wallet(signer.clone())
				.connect_ws(ws)
				.await?,
		);

		let chain_id = provider.get_chain_id().await?;
		let dict = dict::load(&params.chain_dict).context("invalid chain dict")?;
		let currency = dict.get(&chain_id).map(|c| c.currency.clone()).unwrap_or_default();

		Ok(Self {
			network_id: params.network_id,
			url: params.url,
			rpc: provider,
			signer: Arc::new(signer),
			cctp: Default::default(),
			chain_id,
			currency,
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
		(self.currency.decimals as _, self.currency.symbol.as_str())
	}
	/// Funds Connector's account
	async fn faucet(&self, balance: u128) -> Result<()> {
		let ws = WsConnect::new(self.url.clone());
		let provider = ProviderBuilder::new().connect_ws(ws).await?;
		let sponsor = provider
			.get_accounts()
			.await?
			.first()
			.ok_or(anyhow!("Node owns no account"))?
			.to_owned();
		let nonce = provider.get_transaction_count(sponsor).await?;

		let tx = TransactionRequest::default()
			.with_from(sponsor)
			.with_nonce(nonce)
			.with_to(a_addr(self.address()))
			.with_value(U256::from(balance))
			.with_gas_limit(21_000);

		let guard = self.wallet_guard.lock().await;
		let pending_tx = provider.send_transaction(tx).await?;
		drop(guard);
		let tx_hash = pending_tx
			.with_timeout(Some(Duration::from_secs(DEFAULT_TX_TIMEOUT)))
			.get_receipt()
			.await?
			.transaction_hash;
		tracing::info!("Faucet sent {balance} to {}, tx: {tx_hash}", a_addr(self.address()));
		Ok(())
	}
	/// Transfers an amount to an account
	async fn transfer(&self, address: Address32, amount: u128) -> Result<()> {
		let to = a_addr(address);
		let tx = TransactionRequest::default()
			.with_from(self.signer.address())
			.with_to(to)
			.with_value(U256::from(amount));

		let guard = self.wallet_guard.lock().await;
		let pending_tx = self.rpc.send_transaction(WithOtherFields::new(tx)).await?;
		drop(guard);
		let tx_hash = pending_tx
			.with_timeout(Some(Duration::from_secs(60)))
			.get_receipt()
			.await?
			.transaction_hash;
		tracing::info!("Transferred sent {amount} to {to}, tx: {tx_hash}");
		Ok(())
	}
	/// Queries the account balance
	async fn balance(&self, address: Address32) -> Result<u128> {
		Ok(self.rpc.get_balance(a_addr(address)).await?.try_into()?)
	}
	async fn finalized_block(&self) -> Result<u64> {
		self.rpc
			.get_block(BlockId::finalized())
			.await?
			.map(|b| b.header.number)
			.ok_or(anyhow!("failed querying finalized block"))
	}
	/// Stream of finalized block indicies
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send>> {
		Box::pin(FinalizedBlockStream::new(self.rpc.clone()).map(|b| b.header.number))
	}
}

#[async_trait]
impl IConnector for Connector {
	/// Reads gmp messages from the target chain.
	async fn read_events(
		&self,
		gateway: Address32,
		blocks: Range<u64>,
		cctp_info: Option<(Vec<Address32>, String)>,
	) -> Result<Vec<GmpEvent>> {
		let contract = a_addr(gateway);
		let filter = Filter::new()
			.address(contract)
			.from_block(BlockNumberOrTag::Number(blocks.start))
			// NOTE: rust range is end exclusive, whereas ETH RPC is end inclusive
			.to_block(BlockNumberOrTag::Number(blocks.end - 1));
		let logs = self.rpc.get_logs(&filter).await?;

		let mut events = vec![];
		for outer_log in logs {
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
						let log = sol::Gateway::ShardsRegistered::decode_log(&log)?;
						for key in log.keys.iter() {
							events.push(GmpEvent::ShardRegistered(key.clone().into()));
						}
					},
					sol::Gateway::ShardsUnregistered::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardsUnregistered::decode_log(&log)?;
						for key in log.keys.iter() {
							events.push(GmpEvent::ShardUnregistered(key.clone().into()));
						}
					},
					sol::Gateway::GmpCreated::SIGNATURE_HASH => {
						let log = sol::Gateway::GmpCreated::decode_log(&log)?;
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
						if !self.cctp.needs_attestation(&gmp_message, cctp_info.as_ref()) {
							tracing::info!(
								"gmp created: {:?}",
								hex::encode(gmp_message.message_id())
							);
							events.push(GmpEvent::MessageReceived(gmp_message));
						}
					},
					sol::Gateway::GmpExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::GmpExecuted::decode_log(&log)?;
						tracing::info!("gmp executed: {:?}", hex::encode(log.id));
						events.push(GmpEvent::MessageExecuted(log.id.into()));
					},
					sol::Gateway::BatchExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::BatchExecuted::decode_log(&log)?;
						events.push(GmpEvent::BatchExecuted {
							batch_id: log.batch,
							tx_hash: outer_log.transaction_hash.map(|hash| hash.into()),
						});
					},
					_ => {},
				}
			}
		}
		// CCTP calls processing
		while let Some(msg) = self.cctp.pop_attested().await {
			tracing::info!("gmp created: {:?}", hex::encode(msg.message_id()));
			events.push(GmpEvent::MessageReceived(msg));
		}
		Ok(events)
	}
	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		gateway: Address32,
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
		let total_gas = msg.gas().saturating_add(200_000u128);
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

		let gw_call = gw.batchExecute(signature, message);
		let estimated_gas = gw_call.estimate_gas().await.map_err(|err| err.to_string())?;
		let max_gas = std::cmp::max(estimated_gas, gas_limit);

		let receipt = gw_call
			.gas(max_gas)
			.send()
			.await
			.map_err(|err| {
				tracing::info!("failed to submit batch: {:?}", err);
				err.to_string()
			})?
			.with_timeout(Some(Duration::from_secs(DEFAULT_TX_TIMEOUT)))
			.get_receipt()
			.await
			.map_err(|err| err.to_string())?;
		let tx_hash = receipt.transaction_hash;

		if !receipt.inner.inner.is_success() {
			let err = format!("batch {batch} failed with tx: {tx_hash}");
			tracing::error!(err);
			return Err(err.into());
		} else {
			tracing::info!("batch {batch} submitted with tx: {tx_hash}");
		}

		Ok(())
	}
}

#[async_trait]
impl IConnectorAdmin for Connector {
	/// Deploys gateway contract
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
		let proxy_deployed_code = self.rpc.get_code_at(proxy_address).await?;
		if !proxy_deployed_code.is_empty() {
			let block = self.latest_block().await?;
			tracing::info!("Proxy already deployed. Please use redeploy-gateway instead");
			return Ok((t_addr(proxy_address), block.number));
		}
		// deploy gateway
		let gateway_address = self.deploy_gateway_contract(proxy_address, gateway).await?;
		// compute proxy arguments
		let (proxy_address, block) = self
			.deploy_proxy_contract(&config, proxy_address, gateway_address, proxy)
			.await?;

		Ok((t_addr(proxy_address), block))
	}
	/// Redeploys gateway contract
	async fn redeploy_gateway(&self, proxy: Address32, gateway: &[u8]) -> Result<()> {
		let gateway = extract_bytecode(gateway)?;
		let proxy_address = a_addr(proxy);

		let gateway_addr = self.deploy_gateway_contract(proxy_address, gateway).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: gateway_addr,
		};

		let tx = TransactionRequest::default()
			.with_to(proxy_address)
			.with_chain_id(self.rpc.get_chain_id().await?)
			.with_call(&call);

		self.rpc
			.send_transaction(WithOtherFields::new(tx))
			.await?
			.with_timeout(Some(Duration::from_secs(DEFAULT_TX_TIMEOUT)))
			.get_receipt()
			.await?;

		Ok(())
	}
	/// Deploys test contract
	async fn deploy_test(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)> {
		let call = sol::GmpTester::constructorCall { gateway: a_addr(gateway) };
		let mut bytecode = extract_bytecode(tester)?;
		bytecode.extend(call.abi_encode());

		let tx = TransactionRequest::default().with_deploy_code(bytecode);

		let receipt =
			self.rpc.send_transaction(WithOtherFields::new(tx)).await?.get_receipt().await?;
		let contract_address = receipt
			.contract_address()
			.ok_or(anyhow!("Failed to get deployed contract address"))?;
		let block_number = receipt
			.block_number
			.ok_or(anyhow!("Failed to get contract deployement block"))?;

		Ok((t_addr(contract_address), block_number))
	}

	/// Returns gateway admin
	async fn admin(&self, gateway: Address32) -> Result<Address32> {
		let admin_address = self.evm_call(gateway, sol::Gateway::adminCall {}).await?.0;
		Ok(t_addr(admin_address.into()))
	}
	/// Sets gateway admin
	async fn set_admin(&self, gateway: Address32, admin: Address32) -> Result<()> {
		let call = sol::Gateway::setAdminCall { admin: a_addr(admin) };
		let _receipt = self.evm_send(gateway, call, 0).await?;
		Ok(())
	}
	/// Returns registered shard keys
	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>> {
		let keys = self.evm_call(gateway, sol::Gateway::shardsCall {}).await?;
		let keys = keys.into_iter().map(Into::into).collect();
		Ok(keys)
	}
	/// Sets registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address32, keys: &[TssPublicKey]) -> Result<()> {
		let mut shards = keys.iter().copied().map(Into::into).collect::<Vec<TssKey>>();
		shards.sort_by(|a, b| a.xCoord.cmp(&b.xCoord));
		let call = sol::Gateway::setShardsCall { publicKeys: shards };

		let _receipt = self.evm_send(gateway, call, 0).await?;
		Ok(())
	}
	/// Returns gateway routing table
	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>> {
		let routes = self.evm_call(gateway, sol::Gateway::routesCall {}).await?;
		let routes = routes.into_iter().map(Into::into).collect();
		Ok(routes)
	}
	/// Updates an entry in gateway routing table
	async fn set_route(&self, gateway: Address32, route: Route) -> Result<()> {
		let call = sol::Gateway::setRouteCall { info: route.into() };
		let _receipt = self.evm_send(gateway, call, 0).await?;
		Ok(())
	}
	/// Estimates message gas limit
	async fn estimate_message_gas_limit(
		&self,
		contract: Address32,
		src_network: NetworkId,
		src: Address32,
		payload: Vec<u8>,
	) -> Result<u128> {
		let call = sol::IGmpReceiver::onGmpReceivedCall {
			id: [0; 32].into(),
			network: src_network.into(),
			source: src.into(),
			nonce: 0,
			payload: payload.into(),
		};
		let tx = TransactionRequest::default()
			.with_to(a_addr(contract))
			.with_chain_id(self.rpc.get_chain_id().await?)
			.with_call(&call);

		Ok(self.rpc.estimate_gas(WithOtherFields::new(tx)).await? as u128)
	}
	/// Estimates message cost
	async fn estimate_message_cost(
		&self,
		gateway: Address32,
		dest_network: NetworkId,
		gas_limit: u128,
		payload: Vec<u8>,
	) -> Result<u128> {
		let msg = sol::GmpMessage {
			source: [0; 32].into(),
			srcNetwork: 0,
			dest: [0; 20].into(),
			destNetwork: 0,
			gasLimit: 0,
			nonce: 0,
			data: payload.into(),
		};
		let call = sol::Gateway::estimateMessageCostCall {
			networkid: dest_network,
			// abi_encoded_size returns the size without the 4 byte selector
			messageSize: U256::from(msg.abi_encoded_size() + 4),
			gasLimit: U256::from(gas_limit),
		};
		let result = self.evm_call(gateway, call).await?;
		let msg_cost: u128 = result.try_into().map_err(|e| anyhow!("{e}"))?;

		Ok(msg_cost)
	}
	// Sends a message using the test contract
	async fn send_message(
		&self,
		contract: Address32,
		dest_network: NetworkId,
		dest: Address32,
		gas_limit: u128,
		gas_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		let msg = sol::GmpMessage {
			srcNetwork: self.network_id,
			source: contract.into(),
			destNetwork: dest_network,
			dest: a_addr(dest),
			nonce: 0,
			gasLimit: gas_limit as _,
			data: payload.into(),
		};
		tracing::debug!("Sending GMP message: {:#?}", &msg);
		let call = sol::GmpTester::sendMessageCall { msg };
		let receipt = self.evm_send(contract, call, gas_cost).await?;

		receipt
			.inner
			.inner
			.logs()
			.iter()
			.filter(|e| e.topics().contains(&sol::Gateway::GmpCreated::SIGNATURE_HASH))
			.filter_map(|e| sol::Gateway::GmpCreated::decode_log_data(e.data()).ok())
			.map(|e| e.id.into())
			.next()
			.ok_or(anyhow!("Failed to send message"))
	}

	/// Receives messages from test contract
	async fn recv_messages(
		&self,
		contract: Address32,
		blocks: Range<u64>,
	) -> Result<Vec<GmpMessage>> {
		let contract = a_addr(contract);
		let filter = Filter::new()
			.address(contract)
			.from_block(BlockNumberOrTag::Number(blocks.start))
			// NOTE: rust range is end exclusive, whereas ETH RPC is end inclusive
			.to_block(BlockNumberOrTag::Number(blocks.end - 1));

		let logs = self.rpc.get_logs(&filter).await?;

		Ok(logs
			.into_iter()
			.filter(|e| e.topics().contains(&sol::GmpTester::MessageReceived::SIGNATURE_HASH))
			.filter_map(|e| sol::GmpTester::MessageReceived::decode_log_data(e.data()).ok())
			.map(|e| e.msg.into())
			.collect::<Vec<_>>())
	}

	/// Get EIP1559 `max_fee_per_gas` estimate for the connector's chain
	async fn max_fee_per_gas(&self) -> Result<u128> {
		let (fee_estimator, past_blocks, reward_percentile) = match self.chain_id {
			// Polygon
			137 => (Eip1559Estimator::Default, 15, 10.0),
			// BNB
			97 | 56 => (Eip1559Estimator::Custom(Box::new(BEP226)), 1, 5.0),
			// Default
			_ => (Eip1559Estimator::Default, 10, 5.0),
		};

		let block = self.latest_block().await?;
		let base_fee = block.base_fee_per_gas.ok_or(anyhow!("Failed to get latest base fee"))?;

		let rewards = self
			.rpc
			.get_fee_history(past_blocks, BlockNumberOrTag::Latest, &[reward_percentile])
			.await?
			.reward
			.ok_or(anyhow!("Failed to get rewards from fee history"))?;
		Ok(fee_estimator.estimate(base_fee.into(), &rewards).max_fee_per_gas)
	}

	/// Returns gas limit of latest block
	async fn block_gas_limit(&self) -> Result<u64> {
		self.latest_block().await.map(|b| b.gas_limit)
	}

	/// Withdraw gateway funds
	async fn withdraw_funds(
		&self,
		gateway: Address32,
		amount: u128,
		recipient: Address32,
	) -> Result<()> {
		let call = sol::Gateway::withdrawCall {
			amount: U256::from(amount),
			recipient: a_addr(recipient),
			data: vec![].into(),
		};
		let _receipt = self.evm_send(gateway, call, 0).await?;
		Ok(())
	}
	/// Debug a transaction.
	// TODO could be done with alloy as well
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
}

impl Connector {
	async fn evm_call<C: SolCall>(&self, to: Address32, call: C) -> Result<C::Return> {
		let tx = TransactionRequest::default()
			.with_to(a_addr(to))
			.with_chain_id(self.rpc.get_chain_id().await?)
			.with_call(&call);

		let result = self.rpc.call(WithOtherFields::new(tx)).await?;

		Ok(C::abi_decode_returns(&result)?)
	}

	async fn evm_send<C: SolCall>(
		&self,
		to: Address32,
		call: C,
		value: u128,
	) -> Result<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>> {
		let tx = TransactionRequest::default()
			.with_to(a_addr(to))
			.with_chain_id(self.rpc.get_chain_id().await?)
			.with_call(&call)
			.with_value(U256::from(value));

		let _guard = self.wallet_guard.lock().await;

		Ok(self.rpc.send_transaction(WithOtherFields::new(tx)).await?.get_receipt().await?)
	}

	async fn latest_block(&self) -> Result<Header<AnyHeader>> {
		self.rpc
			.get_block(BlockId::latest())
			.await?
			.map(|b| b.header.clone())
			.ok_or(anyhow!("failed querying finalized block"))
	}

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
		let pending_tx = self.rpc.send_transaction(WithOtherFields::new(tx)).await?;
		drop(guard);
		let tx_hash = *pending_tx.tx_hash();
		tracing::debug!("deployment tx: {tx_hash}");

		let receipt = pending_tx.get_receipt().await?;
		tracing::debug!("deployment tx receipt: {receipt:?}");

		let log = receipt
			.inner
			.inner
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
			.with_timeout(Some(Duration::from_secs(DEFAULT_TX_TIMEOUT)))
			.get_receipt()
			.await?
			.transaction_hash;
		tracing::info!("factory deployed with tx {tx_hash}");

		Ok(())
	}

	async fn deploy_gateway_contract(
		&self,
		proxy: Address20,
		mut bytecode: Vec<u8>,
	) -> Result<Address20> {
		let constructor = sol::Gateway::constructorCall {
			network: self.network_id,
			proxy,
		};
		bytecode.extend(constructor.abi_encode());
		let tx = TransactionRequest::default().with_deploy_code(bytecode);

		let _guard = self.wallet_guard.lock().await;
		let receipt =
			self.rpc.send_transaction(WithOtherFields::new(tx)).await?.get_receipt().await?;
		drop(_guard);
		let gateway_address = receipt.contract_address().expect("Failed to get contract address");

		Ok(gateway_address)
	}

	async fn deploy_proxy_contract(
		&self,
		config: &DeploymentConfig,
		proxy_addr: Address20,
		gateway_address: Address20,
		mut bytecode: Vec<u8>,
	) -> Result<(Address20, u64)> {
		// constructor params
		let admin = a_addr(self.address());
		let constructor = sol::GatewayProxy::constructorCall { admin };
		bytecode.extend(constructor.abi_encode());
		// computing signature for security purpose
		let digest = ProxyDigest {
			proxy: proxy_addr,
			implementation: gateway_address,
		}
		.abi_encode();
		let payload: [u8; 32] = Keccak256::digest(digest).into();
		let sig = self.signer.sign_hash_sync(&payload.into())?;
		let arguments = ProxyContext {
			// Ethereum verification uses 27,28 instead of 0,1 for recovery id
			v: sig.v() as u8 + 27,
			r: sig.r().into(),
			s: sig.s().into(),
			implementation: gateway_address,
		}
		.abi_encode();

		let initializer = sol::Gateway::initializeCall {
			admin,
			keys: vec![],
			networks: vec![],
		}
		.abi_encode();

		// Proxy creation
		let call = sol::IUniversalFactory::create2_1Call {
			salt: config.deployment_salt.into(),
			creationCode: bytecode.into(),
			arguments: arguments.into(),
			callback: initializer.into(),
		}
		.abi_encode();

		let (proxy_address, block) = self.deploy_contract_with_factory(config, call).await?;

		if proxy_address != proxy_addr {
			anyhow::bail!(
				"Unable to compute proxy address: expected: {:?}, got {:?}",
				proxy_addr,
				proxy_address
			);
		}
		tracing::info!("proxy deployed at {} {}", proxy_address, block);
		Ok((proxy_address, block))
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

#[derive(Deserialize)]
struct Contract {
	bytecode: Bytecode,
}

#[derive(Deserialize)]
struct Bytecode {
	object: String,
}
