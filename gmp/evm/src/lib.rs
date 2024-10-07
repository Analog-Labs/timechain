use alloy_primitives::{B256, U256};
use alloy_sol_types::{SolCall, SolConstructor, SolEvent};
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use rosetta_client::{
	query::GetLogs, types::AccountIdentifier, AtBlock, CallResult, FilterBlockOption,
	GetTransactionCount, SubmitResult, TransactionReceipt, Wallet,
};
use std::ops::Range;
use std::pin::Pin;
use std::sync::Arc;
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
	addr.copy_from_slice(&address.0 .0);
	addr
}

#[derive(Clone)]
pub struct Connector {
	network_id: NetworkId,
	wallet: Arc<Wallet>,
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
		let json_abi: serde_json::Value = serde_json::from_slice(abi)?;
		let mut contract =
			hex::decode(json_abi["bytecode"]["object"].as_str().unwrap().replace("0x", ""))?;
		contract.extend(constructor.abi_encode());
		let tx_hash = self.wallet.eth_deploy_contract(contract).await?.tx_hash().0;
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash).await?.unwrap();
		let address = tx_receipt.contract_address.unwrap();
		let block_number = tx_receipt.block_number.unwrap();
		Ok((t_addr(address.0.into()), block_number))
	}
}

#[async_trait]
impl IConnectorBuilder for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
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
		Ok(Self {
			network_id: params.network_id,
			wallet,
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
	async fn deploy_gateway(&self, proxy: &[u8], gateway: &[u8]) -> Result<(Address, u64)> {
		let admin = self.address();

		// get deployer's nonce
		let nonce = self.nonce(admin).await?;

		// compute the proxy address
		let proxy_addr = a_addr(admin).create(nonce + 1);
		let proxy_addr = t_addr(proxy_addr);

		// deploy gateway
		let call = sol::Gateway::constructorCall {
			networkId: self.network_id,
			proxy: a_addr(proxy_addr),
		};
		let (gateway_addr, _) = self.deploy_contract(gateway, call).await?;

		// deploy the proxy contract
		let call = sol::GatewayProxy::constructorCall {
			implementation: a_addr(gateway_addr),
		};
		let (actual_addr, block_number) = self.deploy_contract(proxy, call).await?;

		// Check if the proxy address match the expect address
		if proxy_addr != actual_addr {
			anyhow::bail!("Proxy address mismatch, expect {proxy_addr:?}, got {actual_addr:?}");
		}
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
}
