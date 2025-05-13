use crate::cctp::CctpHandler;
use crate::custom::BEP226;
use crate::dict::Currency;
use crate::sol::{Gateway, GatewayProxy, GmpProxy, IGmpReceiver};
use alloy::{
	dyn_abi::DynSolValue,
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
	},
	sol_types::{SolCall, SolConstructor, SolEvent, SolValue},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::{ops::Range, process::Command, sync::Arc, time::Duration};
use time_primitives::{
	Address32, BatchId, ConnectorParams, GatewayMessage, GmpEvent, GmpMessage, Hash, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, SwapPrerequisites,
	TssPublicKey, TssSignature,
};
use tokio::sync::Mutex;

type Address20 = alloy::primitives::Address;

mod cctp;
mod custom;
mod dict;
// some e2e tests use it
pub mod sol;

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
	submitter: Submitter,
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
			cctp: Arc::new(CctpHandler::new()),
			chain_id,
			currency,
			submitter: Submitter::new(Duration::from_secs(60)),
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
		let provider = ProviderBuilder::new().network::<AnyNetwork>().connect_ws(ws).await?;
		let sponsor = provider
			.get_accounts()
			.await?
			.first()
			.ok_or(anyhow!("Node owns no account"))?
			.to_owned();

		let tx = TransactionRequest::default()
			.with_from(sponsor)
			.with_to(a_addr(self.address()))
			.with_value(U256::from(balance));

		let receipt = self.submitter.submit(&provider, tx).await?;
		tracing::info!(
			"faucet sent {balance} to {}, tx: {:?}",
			a_addr(self.address()),
			receipt.transaction_hash()
		);
		Ok(())
	}
	/// Transfers an amount to an account
	async fn transfer(&self, to: Address32, amount: u128) -> Result<()> {
		let tx = TransactionRequest::default().with_to(a_addr(to)).with_value(U256::from(amount));
		let receipt = self.submit(tx).await?;
		tracing::info!(
			"transferred {amount} to {}, tx: {:?}",
			a_addr(to),
			receipt.transaction_hash()
		);
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
		let signature = Gateway::Signature {
			xCoord: sol::u256(&signer[1..33]),
			e: sol::u256(&sig[..32]),
			s: sol::u256(&sig[32..]),
		};
		let ops: Vec<Gateway::GatewayOp> = msg.ops.iter().map(|op| op.clone().into()).collect();
		let message = Gateway::InboundMessage {
			version: 0,
			batchID: batch,
			ops,
		};
		let call = Gateway::batchExecuteCall { signature, message };
		let tx = TransactionRequest::default().with_to(a_addr(gateway)).with_call(&call);
		self.submit(tx).await.map_err(|err| err.to_string())?;
		Ok(())
	}
}

#[async_trait]
impl IConnectorAdmin for Connector {
	/// Deploys proxy contract
	async fn deploy_proxy(&self, proxy: &[u8]) -> Result<(Address32, u64)> {
		let admin = a_addr(self.address());
		let proxy_constructor = GatewayProxy::constructorCall { admin };
		let (proxy_addr, proxy_block) = self.deploy_contract(proxy, proxy_constructor).await?;
		Ok((t_addr(proxy_addr), proxy_block))
	}

	/// Deploys gateway contract
	async fn deploy_gateway(&self, proxy: Address32, gateway: &[u8]) -> Result<()> {
		let proxy = a_addr(proxy);
		let (gateway_addr, _gateway_block) = self
			.deploy_contract(
				gateway,
				Gateway::constructorCall {
					network: self.network_id,
					proxy,
				},
			)
			.await?;

		let call = GatewayProxy::upgradeCall {
			newImplementation: gateway_addr,
		};
		let tx = TransactionRequest::default().with_to(proxy).with_call(&call);
		self.submit(tx).await?;

		Ok(())
	}
	/// Deploys test contract
	async fn deploy_tester(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)> {
		let call = GmpProxy::constructorCall { gateway: a_addr(gateway) };
		let (addr, block) = self.deploy_contract(tester, call).await?;
		Ok((t_addr(addr), block))
	}

	async fn deploy_zenswap(
		&self,
		gateway: Address32,
		zenswap: &[u8],
		zenswap_plugin: &[u8],
		helper_contracts: SwapPrerequisites,
	) -> Result<(Address32, Address32)> {
		let universal_router = a_addr(helper_contracts.universal_router);
		let transmitter = a_addr(helper_contracts.msg_transmitter);
		let permit2 = a_addr(helper_contracts.permit2);
		let messenger = a_addr(helper_contracts.token_messenger);
		let usdc = a_addr(helper_contracts.usdc);

		let plugin_initializer = sol::ZenSwapGmpPlugin::initializeCall {
			_gmpGateway: a_addr(gateway),
			_cctpMessenger: messenger,
			_cctpReceiver: transmitter,
			_usdc: usdc,
			_fee: u256(&[0u8; 32]),
		};

		let zenswap_contructor = sol::ZenSwap::constructorCall {
			_universalRouter: universal_router,
			_permit2: permit2,
		};

		let mut zenswap_bytecode = extract_bytecode(zenswap, Default::default())?;
		zenswap_bytecode.extend(zenswap_contructor.abi_encode());

		// Zenswap deployment
		let tx = TransactionRequest::default().with_deploy_code(zenswap_bytecode);
		let receipt =
			self.rpc.send_transaction(WithOtherFields::new(tx)).await?.get_receipt().await?;
		let zenswap_addr =
			receipt.contract_address.ok_or(anyhow!("Unable to get contract address"))?;

		// Message lib deployment
		let msg_lib_code = hex::decode(sol::CIRCLE_MESSAGE_LIB_BYTECODE)?;
		let tx = TransactionRequest::default().with_deploy_code(msg_lib_code);
		let receipt =
			self.rpc.send_transaction(WithOtherFields::new(tx)).await?.get_receipt().await?;
		let lib_addr = receipt
			.contract_address
			.ok_or(anyhow!("Unable to get message library address"))?;

		// ZenswapPlugin deployment
		let mut replacement_keys = HashMap::new();
		replacement_keys.insert("__$2e72248e36cbd9e27bfc8c16586a2f5547$__", hex::encode(lib_addr));
		let plugin_bytecode = extract_bytecode(zenswap_plugin, replacement_keys)?;
		let tx = TransactionRequest::default().with_deploy_code(plugin_bytecode);
		let receipt =
			self.rpc.send_transaction(WithOtherFields::new(tx)).await?.get_receipt().await?;
		let plugin_address =
			receipt.contract_address.ok_or(anyhow!("Unable to get plugin address"))?;

		//initialize the plugin
		self.evm_send(t_addr(plugin_address), plugin_initializer, 0).await?;
		tracing::info!("zenswap addr: {}", hex::encode(zenswap_addr));
		tracing::info!("zenswap plugin addr: {}", hex::encode(plugin_address));
		Ok((t_addr(zenswap_addr), t_addr(plugin_address)))
	}

	async fn send_swap(
		&self,
		dest: NetworkId,
		dest_name: String,
		src_zenswap_addr: Address32,
		src_plugin: Address32,
		dst_zenswap_addr: Address32,
		dst_plugin: Address32,
		src_contracts: SwapPrerequisites,
		dst_contracts: SwapPrerequisites,
	) -> Result<MessageId> {
		let src_usdc = a_addr(src_contracts.usdc);
		let dest_usdc = a_addr(dst_contracts.usdc);

		let domain_id = chain_to_domain_id(&dest_name)?;
		let params = sol::ZenSwapGmpPlugin::PluginParams {
			destPlugin: a_addr(dst_plugin),
			recipient: a_addr(dst_zenswap_addr),
			fallbackRecipient: a_addr(self.address()),
			cctpDestinationDomain: domain_id,
			gmpDestNetwork: dest,
			gmpGasLimit: 1_000_000,
		};

		// 0.0001 eth
		let amount_u128: u128 = 10000000000000;
		let amount = U256::from(amount_u128);

		let deadline = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("Time went backwards")
			.as_secs()
			+ 3600;

		// src params

		// WRAP_ETH
		//     address The recipient of the WETH
		//     uint256 The amount of ETH to wrap
		let wrap_eth = DynSolValue::Tuple(vec![
			DynSolValue::Address(a_addr(src_contracts.universal_router)),
			DynSolValue::Uint(amount, 256),
		])
		.abi_encode();

		// trade path
		// token_in, fee, token_out
		let src_path_encoded = DynSolValue::Tuple(vec![
			DynSolValue::Address(a_addr(src_contracts.weth)),
			DynSolValue::Uint(U256::from(100), 24),
			DynSolValue::Address(src_usdc),
		])
		.abi_encode_packed();

		// V3_SWAP_EXACT_IN
		//     address The recipient of the output of the trade
		//     uint256 The amount of input tokens for the trade
		//     uint256 The minimum amount of output tokens the user wants
		//     bytes The UniswapV3 encoded path to trade along
		//     bool A flag for whether the input tokens should come from the msg.sender (through Permit2) or whether the funds are already in the UniversalRouter
		let src_swap_data = DynSolValue::Tuple(vec![
			// universal factory address from source
			DynSolValue::Address(a_addr(src_contracts.universal_router)),
			DynSolValue::Uint(U256::from(amount), 256),
			DynSolValue::Uint(U256::from(1), 256),
			DynSolValue::Bytes(src_path_encoded.into()),
			DynSolValue::Bool(false),
		])
		.abi_encode();
		let src_swap_data: Vec<u8> = src_swap_data[32..].into();

		// SWEEP
		//     address The ERC20 token to sweep (or Constants.ETH for ETH)
		//     address The recipient of the sweep
		//     uint256 The minimum required tokens to receive from the sweep
		let sweep_data = DynSolValue::Tuple(vec![
			DynSolValue::Address(src_usdc),
			DynSolValue::Address(a_addr(src_zenswap_addr)),
			DynSolValue::Uint(U256::from(1), 256),
		])
		.abi_encode();

		let src_swap_params = sol::ZenSwap::SwapParams {
			tokenIn: Address20::ZERO,
			tokenOut: src_usdc,
			deadline: U256::from(deadline),
			commands: hex::decode("0b0004").unwrap().into(),
			inputs: vec![wrap_eth.into(), src_swap_data.into(), sweep_data.into()].into(),
		};
		/////////////

		// Dest side, swapping setup.
		let dst_swap_params = sol::ZenSwap::SwapParams {
			tokenIn: dest_usdc,
			tokenOut: dest_usdc,
			deadline: U256::from(deadline),
			commands: vec![].into(),
			inputs: vec![].into(),
		};
		/////////

		let swap_call = sol::ZenSwap::swapSendCall {
			pluginParams: params.abi_encode().into(),
			sourceParams: src_swap_params,
			destParams: dst_swap_params,
			recipient: a_addr(self.address()),
			plugin: a_addr(src_plugin),
			amountIn: amount,
		};
		let gas_cost = self
			.estimate_message_cost(src_plugin, dest, 1_000_000, swap_call.abi_encode())
			.await?;
		tracing::info!("balance of sender: {:?}", self.balance(self.address()).await);
		tracing::info!("Sending cost: {:?}", gas_cost + amount_u128);
		let receipt = self.evm_send(src_zenswap_addr, swap_call, gas_cost + amount_u128).await?;
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

	/// Returns gateway admin
	async fn admin(&self, gateway: Address32) -> Result<Address32> {
		let admin_address = self.call(gateway, sol::Gateway::adminCall {}).await?.0;
		Ok(t_addr(admin_address.into()))
	}
	/// Sets gateway admin
	async fn set_admin(&self, gateway: Address32, admin: Address32) -> Result<()> {
		let call = Gateway::setAdminCall { newAdmin: a_addr(admin) };
		let tx = TransactionRequest::default().with_to(a_addr(gateway)).with_call(&call);
		let _receipt = self.submit(tx).await?;
		Ok(())
	}
	/// Returns registered shard keys
	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>> {
		let keys = self.call(gateway, sol::Gateway::shardsCall {}).await?;
		let keys = keys.into_iter().map(Into::into).collect();
		Ok(keys)
	}
	/// Sets registered shard keys. Overwrites any other keys.
	async fn set_shards(&self, gateway: Address32, keys: &[TssPublicKey]) -> Result<()> {
		let mut shards = keys.iter().copied().map(Into::into).collect::<Vec<Gateway::TssKey>>();
		shards.sort_by(|a, b| a.xCoord.cmp(&b.xCoord));
		let call = Gateway::setShardsCall { publicKeys: shards };
		let tx = TransactionRequest::default().with_to(a_addr(gateway)).with_call(&call);
		let _receipt = self.submit(tx).await?;
		Ok(())
	}
	/// Returns gateway routing table
	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>> {
		let routes = self.call(gateway, Gateway::routesCall {}).await?;
		let routes = routes.into_iter().map(Into::into).collect();
		Ok(routes)
	}
	/// Updates an entry in gateway routing table
	async fn set_route(&self, gateway: Address32, route: Route) -> Result<()> {
		let call = Gateway::setRouteCall { info: route.into() };
		let tx = TransactionRequest::default().with_to(a_addr(gateway)).with_call(&call);
		let _receipt = self.submit(tx).await?;
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
		let call = IGmpReceiver::onGmpReceivedCall {
			id: [0; 32].into(),
			network: src_network.into(),
			source: src.into(),
			nonce: 0,
			payload: payload.into(),
		};
		let tx = TransactionRequest::default().with_to(a_addr(contract)).with_call(&call);

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
		let msg = Gateway::GmpMessage {
			source: [0; 32].into(),
			srcNetwork: 0,
			dest: [0; 20].into(),
			destNetwork: 0,
			gasLimit: 0,
			nonce: 0,
			data: payload.into(),
		};
		let call = Gateway::estimateMessageCostCall {
			networkid: dest_network,
			// abi_encoded_size returns the size without the 4 byte selector
			messageSize: U256::from(msg.abi_encoded_size() + 4),
			gasLimit: U256::from(gas_limit),
		};
		let result = self.call(gateway, call).await?;
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
		let message = GmpProxy::GmpMessage {
			srcNetwork: self.network_id,
			source: contract.into(),
			destNetwork: dest_network,
			dest: a_addr(dest),
			nonce: 0,
			gasLimit: gas_limit as _,
			data: payload.into(),
		};
		tracing::debug!("Sending GMP message: {:#?}", &message);
		let call = GmpProxy::sendMessageCall { message };
		let tx = TransactionRequest::default()
			.with_to(a_addr(contract))
			.with_call(&call)
			.with_value(U256::from(gas_cost));
		let receipt = self.submit(tx).await?;

		receipt
			.inner
			.inner
			.logs()
			.iter()
			.filter(|e| e.topics().contains(&Gateway::GmpCreated::SIGNATURE_HASH))
			.filter_map(|e| Gateway::GmpCreated::decode_log_data(e.data()).ok())
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
			.filter(|e| e.topics().contains(&GmpProxy::MessageReceived::SIGNATURE_HASH))
			.filter_map(|e| GmpProxy::MessageReceived::decode_log_data(e.data()).ok())
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
		let tx = TransactionRequest::default().with_to(a_addr(gateway)).with_call(&call);
		self.submit(tx).await?;
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
	async fn call<C: SolCall>(&self, to: Address32, call: C) -> Result<C::Return> {
		let tx = TransactionRequest::default().with_to(a_addr(to)).with_call(&call);
		let result = self.rpc.call(WithOtherFields::new(tx)).await?;
		tracing::debug!("{result:?}");
		Ok(C::abi_decode_returns(&result)?)
	}

	async fn submit(
		&self,
		tx: TransactionRequest,
	) -> Result<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>> {
		self.submitter.submit(&self.rpc, tx).await
	}

	async fn latest_block(&self) -> Result<Header<AnyHeader>> {
		self.rpc
			.get_block(BlockId::latest())
			.await?
			.map(|b| b.header.clone())
			.ok_or(anyhow!("failed querying finalized block"))
	}

	async fn deploy_contract<C: SolConstructor>(
		&self,
		contract: &[u8],
		constructor: C,
	) -> Result<(Address20, u64)> {
		#[derive(Deserialize)]
		struct Contract {
			bytecode: Bytecode,
		}

		#[derive(Deserialize)]
		struct Bytecode {
			object: String,
		}

		let contract_abi: Contract = serde_json::from_slice(contract)?;
		let mut bytecode = hex::decode(contract_abi.bytecode.object.replace("0x", ""))
			.with_context(|| "Failed to get contract bytecode")?;
		bytecode.extend(constructor.abi_encode());

		let tx = TransactionRequest::default().with_deploy_code(bytecode);
		let receipt = self.submit(tx).await?;

		let contract_address = receipt
			.contract_address()
			.ok_or(anyhow!("Failed to get deployed contract address"))?;
		let block_number = receipt
			.block_number
			.ok_or(anyhow!("Failed to get contract deployement block"))?;
		Ok((contract_address, block_number))
	}
}

#[derive(Clone)]
struct Submitter {
	// Temporary fix to avoid nonce overlap
	wallet_guard: Arc<Mutex<()>>,
	tx_timeout: Duration,
}

impl Submitter {
	fn new(tx_timeout: Duration) -> Self {
		Self {
			tx_timeout,
			wallet_guard: Default::default(),
		}
	}
}

impl Submitter {
	async fn submit(
		&self,
		provider: impl Provider<AnyNetwork>,
		tx: TransactionRequest,
	) -> Result<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>> {
		let guard = self.wallet_guard.lock().await;
		let pending_tx = provider.send_transaction(WithOtherFields::new(tx)).await?;
		drop(guard);
		tracing::info!("tx {:?} submitted", pending_tx.tx_hash());

		let receipt = pending_tx.with_timeout(Some(self.tx_timeout)).get_receipt().await?;
		tracing::info!("tx {:?} confirmed", receipt.transaction_hash());

		if !receipt.inner.inner.is_success() {
			anyhow::bail!("tx {:?} failed", receipt.transaction_hash());
		}
		Ok(receipt)
	}
}
