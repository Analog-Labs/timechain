use crate::bep226::BEP226;
use crate::sol::{ERC1967Proxy, Gateway, GmpProxy, IGmpReceiver};
use alloy::{
	eips::{eip1559::Eip1559Estimation, BlockId, BlockNumberOrTag},
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
	sol_types::{SolCall, SolConstructor, SolEvent},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::{ops::Range, process::Command, sync::Arc, time::Duration};
use time_primitives::{
	Address32, AdminConnector, BatchId, GatewayMessage, GmpEvent, GmpMessage, Hash, IChain,
	IConnect, IConnector, IConnectorAdmin, MessageId, NetworkId, Route, TssPublicKey, TssSignature,
};
use tokio::sync::Mutex;

type Address20 = alloy::primitives::Address;

mod bep226;
// some e2e tests use it
pub mod sol;

fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}

fn t_addr(address: Address20) -> Address32 {
	address.into_word().into()
}

#[derive(Clone)]
pub struct Chain {
	network_id: NetworkId,
	signer: LocalSigner<SigningKey>,
}

impl Chain {
	pub fn new(network_id: NetworkId, mnemonic: &str) -> Result<Self> {
		let signer = MnemonicBuilder::<English>::default().phrase(mnemonic).index(0)?.build()?;
		Ok(Self { network_id, signer })
	}
}

#[async_trait]
impl IConnect for Chain {
	fn chain(&self) -> &dyn IChain {
		self
	}

	async fn connect(&self, url: String) -> Result<Arc<dyn IConnector>> {
		Ok(Arc::new(Connector::new(self.clone(), url).await?))
	}

	async fn connect_admin(&self, url: String) -> Result<Arc<dyn IConnectorAdmin>> {
		Ok(Arc::new(AdminConnector::new(Connector::new(self.clone(), url).await?)))
	}
}

impl IChain for Chain {
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
}

impl std::fmt::Display for Chain {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.network_id.to_string())
	}
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

pub struct Connector {
	chain: Chain,
	url: String,
	rpc: Arc<CProvider>,
	chain_id: u64,
	submitter: Submitter,
}

impl Connector {
	/// Creates a new connector.
	async fn new(chain: Chain, url: String) -> Result<Self> {
		let ws = WsConnect::new(url.clone())
			.with_max_retries(1200)
			.with_retry_interval(Duration::from_secs(3));
		let rpc: Arc<CProvider> = Arc::new(
			ProviderBuilder::new()
				.network::<AnyNetwork>()
				.wallet(chain.signer.clone())
				.connect_ws(ws)
				.await?,
		);
		let chain_id = rpc.get_chain_id().await?;
		tracing::info!("{}: has chain id {}", chain, chain_id);
		let submitter = Submitter::new(chain.network_id(), Duration::from_secs(60));
		Ok(Self {
			chain,
			url,
			rpc,
			chain_id,
			submitter,
		})
	}
}

#[async_trait]
impl IConnector for Connector {
	fn chain(&self) -> &dyn IChain {
		&self.chain
	}

	/// Queries the latest finalized block.
	async fn finalized_block(&self) -> Result<u64> {
		self.rpc
			.get_block(BlockId::finalized())
			.await?
			.map(|b| b.header.number)
			.ok_or(anyhow!("failed querying finalized block"))
	}

	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Address32, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
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
					sol::Gateway::ShardRegistered::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardRegistered::decode_log(&log)?;
						events.push(GmpEvent::ShardRegistered(log.key.clone().into()));
					},
					sol::Gateway::ShardRevoked::SIGNATURE_HASH => {
						let log = sol::Gateway::ShardRevoked::decode_log(&log)?;
						events.push(GmpEvent::ShardUnregistered(log.key.clone().into()));
					},
					sol::Gateway::GmpCreated::SIGNATURE_HASH => {
						let log = sol::Gateway::GmpCreated::decode_log(&log)?;
						let gmp_message = GmpMessage {
							src_network: self.chain.network_id,
							dest_network: log.destinationNetwork,
							src: log.source.into(),
							dest: t_addr(log.destinationAddress),
							nonce: log.nonce,
							gas_limit: log.gasLimit as _,
							bytes: log.data.data.into(),
						};
						tracing::info!(
							"{}: gmp created: {:?}",
							self.chain,
							hex::encode(gmp_message.message_id())
						);
						events.push(GmpEvent::MessageReceived(gmp_message));
					},
					sol::Gateway::GmpExecuted::SIGNATURE_HASH => {
						let log = sol::Gateway::GmpExecuted::decode_log(&log)?;
						tracing::info!("{}: gmp executed: {:?}", self.chain, hex::encode(log.id));
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
		Ok(events)
	}

	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		gateway: Address32,
		batch: BatchId,
		msg: GatewayMessage,
		gas_price: u128,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		let signature = Gateway::Signature {
			xCoord: sol::u256(&signer[1..33]),
			e: sol::u256(&sig[..32]),
			s: sol::u256(&sig[32..]),
		};
		let ops: Vec<Gateway::GatewayOp> = msg.ops.iter().map(|op| op.clone().into()).collect();
		let batch = Gateway::Batch {
			version: 0,
			batchId: batch,
			ops,
		};
		let call = Gateway::executeCall { signature, batch };
		let tx = TransactionRequest::default()
			.with_to(a_addr(gateway))
			.with_call(&call)
			.with_gas_price(gas_price);
		self.submit(tx).await.map_err(|err| err.to_string())?;
		Ok(())
	}

	/// Get EIP1559 `max_fee_per_gas` estimate for the connector's chain
	async fn gas_price(&self) -> Result<u128> {
		self.estimate_eip1559_fees().await.map(|e| e.max_fee_per_gas)
	}
}

#[async_trait]
impl IConnectorAdmin for Connector {
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
			.with_to(a_addr(self.chain.address()))
			.with_value(U256::from(balance));
		let receipt = self.submitter.submit(&provider, tx).await?;

		tracing::info!(
			"{}: faucet sent {balance} to {}, tx: {:?}",
			self.chain,
			a_addr(self.chain.address()),
			receipt.transaction_hash()
		);
		Ok(())
	}

	/// Transfers an amount to an account
	async fn transfer(&self, to: Address32, amount: u128) -> Result<()> {
		let tx = TransactionRequest::default().with_to(a_addr(to)).with_value(U256::from(amount));
		let receipt = self.submit(tx).await?;
		tracing::info!(
			"{}: transferred {amount} to {}, tx: {:?}",
			self.chain,
			a_addr(to),
			receipt.transaction_hash()
		);
		Ok(())
	}

	/// Queries the account balance
	async fn balance(&self, address: Address32) -> Result<u128> {
		Ok(self.rpc.get_balance(a_addr(address)).await?.try_into()?)
	}

	/// Deploys proxy contract
	async fn deploy_gateway(&self, proxy: &[u8], gateway: &[u8]) -> Result<(Address32, u64)> {
		let (gateway_addr, _gateway_block) =
			self.deploy_contract(gateway, Gateway::constructorCall {}).await?;
		let initialize = Gateway::initializeCall {
			_networkId: self.chain.network_id,
		};
		let proxy_constructor = ERC1967Proxy::constructorCall {
			implementation: gateway_addr,
			_data: initialize.abi_encode().into(),
		};
		let (proxy_addr, proxy_block) = self.deploy_contract(proxy, proxy_constructor).await?;
		Ok((t_addr(proxy_addr), proxy_block))
	}

	/// Redeploys gateway contract
	async fn redeploy_gateway(&self, proxy: Address32, gateway: &[u8]) -> Result<()> {
		let proxy = a_addr(proxy);
		let (gateway_addr, _gateway_block) =
			self.deploy_contract(gateway, Gateway::constructorCall {}).await?;

		let call = Gateway::upgradeToAndCallCall {
			newImplementation: gateway_addr,
			data: [].into(),
		};
		let tx = TransactionRequest::default().with_to(proxy).with_call(&call);
		self.submit(tx).await?;

		Ok(())
	}

	/// Contract bytecode matches
	async fn contract_bytecode_matches(&self, address: Address32, bytecode: &[u8]) -> Result<bool> {
		let address = a_addr(address);
		let bytecode = read_bytecode(bytecode)?;
		let code = self.rpc.get_code_at(address).await?;
		Ok(code.starts_with(&bytecode))
	}

	/// Proxy implementation address
	async fn implementation(&self, proxy: Address32) -> Result<Address32> {
		let proxy = a_addr(proxy);
		let uint = self
			.rpc
			.get_storage_at(proxy, U256::from_be_bytes(sol::IMPLEMENTATION_SLOT))
			.await?;
		Ok(uint.to_be_bytes::<32>())
	}

	/// Deploys test contract
	async fn deploy_tester(&self, gateway: Address32, tester: &[u8]) -> Result<(Address32, u64)> {
		let call = GmpProxy::constructorCall { gateway: a_addr(gateway) };
		let (addr, block) = self.deploy_contract(tester, call).await?;
		Ok((t_addr(addr), block))
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
	async fn set_shards(
		&self,
		gateway: Address32,
		register: &[(TssPublicKey, u16)],
		revoke: &[(TssPublicKey, u16)],
	) -> Result<()> {
		let register = register.iter().copied().map(Into::into).collect::<Vec<Gateway::TssKey>>();
		let revoke = revoke.iter().copied().map(Into::into).collect::<Vec<Gateway::TssKey>>();
		let call = Gateway::setShardsCall { register, revoke };
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

	/// Updates the prices of all routes.
	async fn set_prices(&self, gateway: Address32, prices: &[f64]) -> Result<()> {
		let call = Gateway::setPricesCall {
			prices: prices.iter().copied().map(Gateway::GasPrice::from).collect(),
		};
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
	) -> Result<u64> {
		let call = IGmpReceiver::onGmpReceivedCall {
			id: [0; 32].into(),
			network: src_network,
			source: src.into(),
			nonce: 0,
			payload: payload.into(),
		};
		let tx = TransactionRequest::default().with_to(a_addr(contract)).with_call(&call);
		Ok(self.rpc.estimate_gas(WithOtherFields::new(tx)).await?)
	}

	/// Estimates message cost
	async fn estimate_message_cost(
		&self,
		gateway: Address32,
		dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
	) -> Result<u128> {
		let call = Gateway::estimateMessageCostCall {
			network: dest_network,
			messageSize: U256::from(msg_size),
			gasLimit: gas_limit,
		};
		let result = self.call(gateway, call).await?;
		let msg_cost: u128 = result.try_into().map_err(|e| anyhow!("{e}"))?;
		Ok(msg_cost)
	}

	/// Sends a message using the test contract
	async fn send_messages(
		&self,
		contract: Address32,
		dest_network: NetworkId,
		dest: Address32,
		gas_limit: u64,
		msg_cost: u128,
		payload: Vec<u8>,
		amplification: u16,
	) -> Result<Vec<MessageId>> {
		let message = GmpProxy::GmpMessage {
			srcNetwork: self.chain.network_id,
			source: contract.into(),
			destNetwork: dest_network,
			dest: a_addr(dest),
			nonce: 0,
			gasLimit: gas_limit as _,
			data: payload.into(),
		};
		anyhow::ensure!(amplification > 0);
		tracing::debug!("{}: sending GMP message: {:#?}", self.chain, &message);
		let call = GmpProxy::sendMessagesCall { message, amplification };
		let tx = TransactionRequest::default()
			.with_to(a_addr(contract))
			.with_call(&call)
			.with_value(U256::from(msg_cost * amplification as u128));
		let receipt = self.submit(tx).await?;

		let msgs: Vec<_> = receipt
			.inner
			.inner
			.logs()
			.iter()
			.filter(|e| e.topics().contains(&Gateway::GmpCreated::SIGNATURE_HASH))
			.filter_map(|e| Gateway::GmpCreated::decode_log_data(e.data()).ok())
			.map(|e| e.id.into())
			.collect();
		anyhow::ensure!(msgs.len() == amplification as usize, "failed to send messages");
		Ok(msgs)
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
			std::env::var("EVM_GATEWAY_DIR").context("failed to find EVM_GATEWAY_DIR")?;
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
	/// Get EIP1559 estimate for the connector's chain
	async fn estimate_eip1559_fees(&self) -> Result<Eip1559Estimation> {
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
		Ok(fee_estimator.estimate(base_fee.into(), &rewards))
	}

	async fn call<C: SolCall>(&self, to: Address32, call: C) -> Result<C::Return> {
		let tx = TransactionRequest::default().with_to(a_addr(to)).with_call(&call);
		let result = self.rpc.call(WithOtherFields::new(tx)).await?;
		tracing::debug!(
			"{}: eth_call to: {} result: {result:?}",
			self.chain,
			a_addr(to).to_string(),
		);
		Ok(C::abi_decode_returns(&result)?)
	}

	async fn submit(
		&self,
		tx: TransactionRequest,
	) -> Result<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>> {
		let estimate = self.estimate_eip1559_fees().await?;
		let tx = tx
			.with_max_fee_per_gas(estimate.max_fee_per_gas)
			.with_max_priority_fee_per_gas(estimate.max_priority_fee_per_gas);
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
		let mut bytecode = read_bytecode(contract)?;
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

fn read_bytecode(contract: &[u8]) -> Result<Vec<u8>> {
	#[derive(Deserialize)]
	struct Contract {
		bytecode: Bytecode,
	}

	#[derive(Deserialize)]
	struct Bytecode {
		object: String,
	}

	let contract_abi: Contract = serde_json::from_slice(contract)?;
	hex::decode(contract_abi.bytecode.object.replace("0x", ""))
		.with_context(|| "Failed to get contract bytecode")
}

#[derive(Clone)]
struct Submitter {
	network: NetworkId,
	// Temporary fix to avoid nonce overlap
	wallet_guard: Arc<Mutex<()>>,
	tx_timeout: Duration,
}

impl Submitter {
	fn new(network: NetworkId, tx_timeout: Duration) -> Self {
		Self {
			network,
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
		tracing::info!("{}: tx {:?} submitted", self.network, pending_tx.tx_hash());

		let receipt = pending_tx.with_timeout(Some(self.tx_timeout)).get_receipt().await?;
		tracing::info!("{}: tx {:?} confirmed", self.network, receipt.transaction_hash());

		if !receipt.inner.inner.is_success() {
			anyhow::bail!("{}: tx {:?} failed", self.network, receipt.transaction_hash());
		}
		Ok(receipt)
	}
}
