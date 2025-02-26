use alloy_primitives::{B256, U256};
use alloy_sol_types::{SolCall, SolConstructor, SolEvent, SolValue};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use rosetta_client::{
	query::GetLogs,
	rosetta_ethereum_backend::{jsonrpsee::Adapter, EthereumRpc},
	rosetta_server::ws::{default_client, DefaultClient},
	rosetta_server_ethereum::utils::{
		DefaultFeeEstimatorConfig, EthereumRpcExt, PolygonFeeEstimatorConfig,
	},
	types::AccountIdentifier,
	AtBlock, Blockchain, CallResult, FilterBlockOption, SubmitResult, TransactionReceipt, Wallet,
};
use serde::Deserialize;
use sha3::{Digest, Keccak256};
use sol::{u256, TssKey};
use std::ops::Range;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;
use time_primitives::{
	Address, BatchId, ConnectorParams, Gateway, GatewayMessage, GmpEvent, GmpMessage, Hash, IChain,
	IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route, TssPublicKey,
	TssSignature,
};
use tokio::sync::Mutex;

use crate::sol::CCTP;
use crate::sol::{ProxyContext, ProxyDigest};

type AlloyAddress = alloy_primitives::Address;
type CctpRetryCount = u8;
const MAX_CCTP_RETRY: CctpRetryCount = 3;

pub(crate) mod sol;

fn a_addr(address: Address) -> AlloyAddress {
	let address: [u8; 20] = address[12..32].try_into().unwrap();
	AlloyAddress::from(address)
}

fn t_addr(address: alloy_primitives::Address) -> Address {
	let mut addr = [0; 32];
	addr[12..32].copy_from_slice(&address.0[..]);
	addr
}

#[derive(Clone)]
pub struct Connector {
	network_id: NetworkId,
	wallet: Arc<Wallet>,
	backend: Adapter<DefaultClient>,
	url: String,
	cctp_sender: Option<Vec<Address>>,
	cctp_attestation: Option<String>,
	cctp_queue: Arc<Mutex<Vec<(GmpMessage, CctpRetryCount)>>>,
	// Temporary fix to avoid nonce overlap
	wallet_guard: Arc<Mutex<()>>,
}

impl Connector {
	async fn raw_evm_call(
		&self,
		contract: [u8; 20],
		call: Vec<u8>,
		amount: u128,
		nonce: Option<u64>,
		gas_limit: Option<u64>,
	) -> Result<(Vec<u8>, TransactionReceipt, [u8; 32])> {
		let result = self.wallet.eth_send_call(contract, call, amount, nonce, gas_limit).await?;
		let (result, receipt, tx_hash) = match result {
			SubmitResult::Executed { result, receipt, tx_hash } => (result, receipt, tx_hash),
			SubmitResult::Timeout { tx_hash } => {
				anyhow::bail!("tx 0x{} timed out", hex::encode(tx_hash))
			},
		};
		let result = match result {
			CallResult::Success(result) => {
				tracing::info!("tx 0x{} succeeded", hex::encode(tx_hash));
				result
			},
			CallResult::Revert(reason) => {
				anyhow::bail!(
					"tx 0x{} reverted because {}",
					hex::encode(tx_hash),
					hex::encode(reason)
				);
			},
			CallResult::Error => anyhow::bail!("tx 0x{} failed", hex::encode(tx_hash)),
		};
		Ok((result, receipt, tx_hash.into()))
	}

	async fn evm_call<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		amount: u128,
		nonce: Option<u64>,
		gas_limit: Option<u64>,
	) -> Result<(T::Return, TransactionReceipt, [u8; 32])> {
		let contract: [u8; 20] = contract[12..32].try_into().unwrap();
		let (result, receipt, tx_hash) =
			self.raw_evm_call(contract, call.abi_encode(), amount, nonce, gas_limit).await?;
		Ok((T::abi_decode_returns(&result, true)?, receipt, tx_hash))
	}

	async fn evm_view<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		block: Option<u64>,
	) -> Result<T::Return> {
		let contract: [u8; 20] = contract[12..32].try_into().unwrap();
		let block: AtBlock = if let Some(block) = block { block.into() } else { AtBlock::Latest };
		let result = self.wallet.eth_view_call(contract, call.abi_encode(), block).await?;
		let CallResult::Success(result) = result else { anyhow::bail!("{:?}", result) };
		Ok(T::abi_decode_returns(&result, true)?)
	}

	async fn deploy_contract(
		&self,
		mut bytecode: Vec<u8>,
		constructor: impl SolConstructor,
	) -> Result<(Address, u64)> {
		bytecode.extend(constructor.abi_encode());
		let tx_hash = self.wallet.eth_deploy_contract(bytecode).await?.tx_hash();
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash.0).await?.unwrap();
		let address = tx_receipt.contract_address.unwrap();
		let block_number = tx_receipt.block_number.unwrap();
		tracing::info!(
			"contract deployed at {:?} in block {} with tx {:?}",
			address,
			block_number,
			tx_hash
		);
		Ok((t_addr(address.0.into()), block_number))
	}

	///
	/// init_code == contract_bytecode + contractor_code
	async fn deploy_contract_with_factory(
		&self,
		config: &DeploymentConfig,
		call: Vec<u8>,
	) -> Result<(AlloyAddress, u64)> {
		let factory_address = a_addr(self.parse_address(&config.factory_address)?).0 .0;
		let (_, receipt, tx_hash) =
			self.raw_evm_call(factory_address, call, 0, None, Some(20_000_000)).await?;
		tracing::debug!("{receipt:?}");
		let log = receipt
			.logs
			.iter()
			.find(|log| log.address.as_bytes() == factory_address)
			.with_context(|| format!("tx {} logs not found", hex::encode(tx_hash)))?;
		let topic = log
			.topics
			.first()
			.with_context(|| format!("tx {} topic not found", hex::encode(tx_hash)))?
			.as_bytes();
		let contract_address = AlloyAddress::from_slice(&topic[12..]);
		Ok((contract_address, receipt.block_number.unwrap()))
	}

	async fn deploy_factory_contract(&self, config: &DeploymentConfig) -> Result<()> {
		let deployer_address = self.parse_address(&config.factory_deployer)?;

		// Step1: fund 0x908064dE91a32edaC91393FEc3308E6624b85941
		self.transfer(deployer_address, config.required_balance).await?;

		//Step2: load transaction from config
		let tx = hex::decode(config.raw_tx.strip_prefix("0x").unwrap_or(&config.raw_tx))?;

		//Step3: send eth_rawTransaction
		let tx_hash = self.backend.send_raw_transaction(tx.into()).await?;

		tracing::info!("factory deployed with tx {:?}", tx_hash);
		Ok(())
	}

	async fn deploy_gateway_contract(
		&self,
		config: &DeploymentConfig,
		proxy: AlloyAddress,
		mut bytecode: Vec<u8>,
	) -> Result<AlloyAddress> {
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
		proxy_addr: AlloyAddress,
		gateway_address: AlloyAddress,
		mut bytecode: Vec<u8>,
	) -> Result<(AlloyAddress, u64)> {
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
		let sig = self.wallet.sign_prehashed(&payload)?.to_bytes();
		debug_assert!(sig.len() == 65);
		let r: [u8; 32] = sig[0..32].try_into()?;
		let s: [u8; 32] = sig[32..64].try_into()?;
		let v = sig[64];
		let arguments = ProxyContext {
			// Ethereum verification uses 27,28 instead of 0,1 for recovery id
			v: v + 27,
			r: r.into(),
			s: s.into(),
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
		tracing::info!("proxy deployed at {}", proxy_address);
		Ok((proxy_address, block))
	}

	async fn process_cctp_msg(&self, msg: &mut GmpMessage) -> Result<(), CctpError> {
		let payload = msg.bytes.clone();
		let mut cctp_payload =
			CCTP::abi_decode(&payload, false).map_err(|_| CctpError::InvalidPayload)?;
		if cctp_payload.version != 0 {
			return Err(CctpError::InvalidVersion);
		}
		let burn_message: Vec<u8> = cctp_payload.message.clone().into();
		let burn_hash: [u8; 32] = sha3::Keccak256::digest(&burn_message).into();
		let attestation_response = self.get_cctp_attestation(burn_hash).await?;
		let signature =
			attestation_response.attestation.clone().ok_or(CctpError::AttestationResponse)?;
		let signature = signature.strip_prefix("0x").unwrap_or(&signature);
		let attestation = hex::decode(signature).map_err(|_| CctpError::InvalidSignature)?;
		cctp_payload.attestation = attestation.into();
		msg.bytes = cctp_payload.abi_encode();
		Ok(())
	}

	async fn get_cctp_attestation(
		&self,
		burn_hash: [u8; 32],
	) -> Result<AttestationResponse, CctpError> {
		let uri = self.cctp_attestation.as_deref().ok_or(CctpError::AttestationDisabled)?;
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
		for (mut msg, mut retry_count) in msgs {
			match self.process_cctp_msg(&mut msg).await {
				Ok(()) => attested_msgs.push(msg),
				Err(CctpError::AttestationPending) => {
					retry_count += 1;
					if retry_count >= MAX_CCTP_RETRY {
						tracing::info!("Dropping Cctp message: {msg:?} with count: {retry_count}",);
					} else {
						tracing::info!("Attestation is pending for msg: {:?}", msg);
						queue.push((msg, retry_count));
					}
				},
				Err(error) => {
					tracing::error!("Failed to process cctp message: {:?}: {:?}", msg, error);
				},
			}
		}

		if !queue.is_empty() {
			tracing::info!("{} Cctp messages have pending attestations.", queue.len());
		}
		attested_msgs
	}

	async fn deploy_cctp_contract(
		&self,
		additional_params: &[u8],
		gateway: Address,
		tester: &[u8],
	) -> Result<AlloyAddress> {
		let config: DeploymentConfig = serde_json::from_slice(additional_params)?;
		let mut bytecode = extract_bytecode(tester)?;
		let constructor = sol::GmpTester::constructorCall { gateway: a_addr(gateway) };
		let factory = a_addr(self.parse_address(&config.factory_address)?);
		let tester_addr = compute_create2_address(
			factory.into(),
			config.deployment_salt,
			&bytecode,
			constructor.clone(),
		)?;
		bytecode.extend(constructor.abi_encode());
		let is_tester_deployed =
			self.backend.get_code(tester_addr.0 .0.into(), AtBlock::Latest).await?;
		if !is_tester_deployed.is_empty() {
			Ok(tester_addr)
		} else {
			let call = sol::IUniversalFactory::create2_0Call {
				salt: config.deployment_salt.into(),
				creationCode: bytecode.into(),
			}
			.abi_encode();
			let (addr, _) = self.deploy_contract_with_factory(&config, call).await?;
			Ok(addr)
		}
	}

	async fn get_gateway_nonce(&self, gateway: Address, account: Address) -> Result<u64> {
		let call = sol::Gateway::nonceOfCall { account: a_addr(account) };
		let result = self.evm_call(gateway, call, 0, None, None).await?;
		Ok(result.0._0)
	}
}

#[async_trait]
impl IConnectorBuilder for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		let (blockchain, private_key) = if params.blockchain == "anvil" {
			let private_key = hex_literal::hex![
				"ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
			];
			(Blockchain::Ethereum, Some(private_key))
		} else {
			(params.blockchain.parse()?, None)
		};
		let wallet = Arc::new(
			Wallet::new(blockchain, &params.network, &params.url, &params.mnemonic, private_key)
				.await?,
		);
		let client = default_client(&params.url, None)
			.await
			.with_context(|| "Cannot get ws client for url: {url}")?;
		let adapter = Adapter(client);
		let cctp_sender: Result<Option<Vec<Address>>> = params
			.cctp_sender
			.map(|items| {
				items
					.split(',')
					.map(|s| s.trim())
					.map(|item| {
						let clean_hex = item.strip_prefix("0x").unwrap_or(item);
						let bytes = hex::decode(clean_hex)
							.map_err(|e| anyhow::anyhow!("Hex decode error: {}", e))?;
						let arr: Address = bytes
							.try_into()
							.map_err(|_| anyhow::anyhow!("Invalid address length"))?;
						Ok(arr)
					})
					.collect::<Result<Vec<_>>>()
			})
			.transpose();
		let cctp_sender = cctp_sender?;

		let connector = Self {
			network_id: params.network_id,
			wallet,
			backend: adapter,
			url: params.url,
			cctp_sender,
			cctp_attestation: params.cctp_attestation,
			cctp_queue: Default::default(),
			wallet_guard: Default::default(),
		};
		Ok(connector)
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
		let address: AlloyAddress = address.parse()?;
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
		let config = self.wallet.config();
		(config.currency_decimals, config.currency_symbol)
	}
	/// Uses a faucet to fund the account when possible.
	async fn faucet(&self, balance: u128) -> Result<()> {
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
	async fn finalized_block(&self) -> Result<u64> {
		Ok(self.wallet.status().await?.index)
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
				topics: vec![],
				block: FilterBlockOption::Range {
					from_block: Some(blocks.start.into()),
					// Evm fetches logs from both blocks that is provided in range. This makes end block exclusive.
					to_block: Some((blocks.end - 1).into()),
				},
			})
			.await?;
		let mut events = vec![];
		for outer_log in logs {
			let topics =
				outer_log.topics.iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
			let log = alloy_primitives::Log::new(
				a_addr(gateway),
				topics,
				outer_log.data.0.to_vec().into(),
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
						if let Some(senders) = &self.cctp_sender {
							if senders.contains(&gmp_message.src) {
								let mut cctp_queue = self.cctp_queue.lock().await;
								cctp_queue.push((gmp_message.clone(), 0));
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
		let signature = sol::Signature {
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
		let ops: Vec<sol::GatewayOp> = msg.ops.iter().map(|op| op.clone().into()).collect();
		let call = sol::Gateway::batchExecuteCall {
			signature,
			message: sol::InboundMessage {
				version: 0,
				batchID: batch,
				ops,
			},
		};
		tracing::info!("submitting batch {batch} with {gas_limit} gas");
		let _guard = self.wallet_guard.lock().await;
		self.evm_call(gateway, call, 0, None, Some(gas_limit)).await.map_err(|err| {
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
	) -> Result<(Address, u64)> {
		let config: DeploymentConfig = serde_json::from_slice(additional_params)?;
		let proxy = extract_bytecode(proxy)?;
		let gateway = extract_bytecode(gateway)?;

		// deploy factory
		let factory_address = a_addr(self.parse_address(&config.factory_address)?).0 .0;
		let is_factory_deployed =
			self.backend.get_code(factory_address.into(), AtBlock::Latest).await?;
		if is_factory_deployed.is_empty() {
			self.deploy_factory_contract(&config).await?;
		}

		// proxy address computation
		let admin = a_addr(self.address());
		let constructor = sol::GatewayProxy::constructorCall { admin };
		let proxy_addr =
			compute_create2_address(factory_address, config.deployment_salt, &proxy, constructor)?;

		// check if proxy is deployed
		let is_proxy_deployed =
			self.backend.get_code(proxy_addr.0 .0.into(), AtBlock::Latest).await?;
		if !is_proxy_deployed.is_empty() {
			tracing::debug!("Proxy already deployed, Please upgrade the gateway contract");
			return Ok((t_addr(proxy_addr), 0));
		}

		// gateway deployment
		let gateway_addr = self.deploy_gateway_contract(&config, proxy_addr, gateway).await?;

		// compute proxy arguments
		let (proxy_address, block) =
			self.deploy_proxy_contract(&config, proxy_addr, gateway_addr, proxy).await?;

		Ok((t_addr(proxy_address), block))
	}

	/// Redeploys the gateway contract.
	async fn redeploy_gateway(
		&self,
		additional_params: &[u8],
		proxy: Address,
		gateway: &[u8],
	) -> Result<()> {
		let config: DeploymentConfig = serde_json::from_slice(additional_params)?;
		let gateway = extract_bytecode(gateway)?;

		let gateway_addr = self.deploy_gateway_contract(&config, a_addr(proxy), gateway).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: gateway_addr,
		};
		self.evm_call(proxy, call, 0, None, None).await?;
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
		self.evm_call(gateway, call, 0, None, None).await?;
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
		let mut shards = keys.iter().copied().map(Into::into).collect::<Vec<TssKey>>();
		shards.sort_by(|a, b| a.xCoord.cmp(&b.xCoord));
		let call = sol::Gateway::setShardsCall { publicKeys: shards };
		self.evm_call(gateway, call, 0, None, None).await?;
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
		let call = sol::Gateway::setRouteCall { info: route.into() };
		self.evm_call(gateway, call, 0, None, None).await?;
		Ok(())
	}
	/// Estimates the message gas limit.
	async fn estimate_message_gas_limit(
		&self,
		contract: Address,
		src_network: NetworkId,
		src: Address,
		payload: Vec<u8>,
	) -> Result<u128> {
		let call = sol::IGmpReceiver::onGmpReceivedCall {
			id: [0; 32].into(),
			network: src_network.into(),
			source: src.into(),
			nonce: 0,
			payload: payload.into(),
		};
		let gas_limit = self
			.wallet
			.eth_send_call_estimate_gas(a_addr(contract).into(), call.abi_encode(), 0)
			.await?;
		Ok(gas_limit)
	}
	/// Estimates the message cost.
	async fn estimate_message_cost(
		&self,
		gateway: Address,
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
		let result = self.evm_view(gateway, call, None).await?;
		let msg_cost: u128 = result._0.try_into().unwrap();
		Ok(msg_cost)
	}

	/// Deploys a test contract.
	async fn deploy_test(
		&self,
		additional_params: &[u8],
		gateway: Address,
		tester: &[u8],
	) -> Result<(Address, u64)> {
		let bytecode = extract_bytecode(tester)?;
		let cctp_contract = self.deploy_cctp_contract(additional_params, gateway, tester).await?;
		tracing::info!("CCTP contract deployed at: {:?}", hex::encode(cctp_contract));
		self.deploy_contract(bytecode, sol::GmpTester::constructorCall { gateway: a_addr(gateway) })
			.await
	}

	// Sends a message using the test contract.
	async fn send_message(
		&self,
		contract: Address,
		dest_network: NetworkId,
		dest: Address,
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
		let call = sol::GmpTester::sendMessageCall { msg };
		let result = self.evm_call(contract, call, gas_cost, None, None).await?;
		let id: MessageId = *result.0._0;
		Ok(id)
	}

	async fn send_cctp_message(
		&self,
		gateway: Address,
		contract: Address,
		dest_network: NetworkId,
		dest: Address,
		gas_limit: u128,
		gas_cost: u128,
	) -> Result<MessageId> {
		let cctp_msg_data = "0000000000000000000000060000000000040CDD0000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000009F3B8679C73C2FEF8B59B4F3444D4E156FB70AA50000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001C7D4B196CB0C7B01D743FBC6116A902379C723800000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25000000000000000000000000000000000000000000000000000000000098968000000000000000000000000033A2838EABD69A081CBEBE3F11DED4086C1CFC25";
		let msg_data =
			hex::decode(cctp_msg_data).expect("Unable to create msg data from dummy cctp msg");
		let mut cctp_msg = sol::CCTP {
			version: 0,
			localMessageTransmitter: a_addr(contract),
			localMinter: a_addr([0u8; 32]),
			amount: U256::from(0),
			destinationDomain: dest_network as u32,
			mintRecipient: dest.into(),
			burnToken: a_addr([0u8; 32]),
			nonce: 0,
			attestation: Vec::new().into(),
			message: msg_data.clone().into(),
			extraData: Vec::new().into(),
		};
		let nonce = self.get_gateway_nonce(gateway, contract).await?;
		let mut msg = sol::GmpMessage {
			srcNetwork: self.network_id,
			source: contract.into(),
			destNetwork: dest_network,
			dest: a_addr(dest),
			nonce,
			gasLimit: gas_limit as _,
			data: cctp_msg.clone().abi_encode().into(),
		};
		let call = sol::GmpTester::sendMessageCall { msg: msg.clone() };
		let _ = self.evm_call(contract, call, gas_cost, None, None).await?;

		// Computing valid message id for CCTP requires adding attestation in the cctp struct and then compute message_id
		// Since after getting attestation the message_id of the gmp message changes we get attestation in start to match the later message_id.
		let burn_hash: [u8; 32] = sha3::Keccak256::digest(&msg_data).into();
		let response = self.get_cctp_attestation(burn_hash).await?;
		let attestation =
			response.attestation.ok_or(anyhow::anyhow!("Failed to get msg attestation"))?;
		let attestation = attestation.strip_prefix("0x").unwrap_or(&attestation);
		let attestation_bytes = hex::decode(attestation).unwrap();
		cctp_msg.attestation = attestation_bytes.into();
		msg.data = cctp_msg.abi_encode().into();
		let message_id = Into::<GmpMessage>::into(msg).message_id();
		Ok(message_id)
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
				topics: vec![],
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
			for topic in log.topics() {
				let sol::GmpTester::MessageReceived::SIGNATURE_HASH = *topic else {
					continue;
				};
				let log = sol::GmpTester::MessageReceived::decode_log(&log, true)?;
				let msg: GmpMessage = log.msg.clone().into();
				msgs.push(msg);
			}
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
		self.evm_call(gateway, call, 0, None, None).await?;
		Ok(())
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

fn compute_create2_address(
	factory_address: [u8; 20],
	salt: [u8; 32],
	bytecode: &[u8],
	constructor: impl SolConstructor,
) -> Result<AlloyAddress> {
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

	Ok(AlloyAddress::from_slice(&proxy_hashed[12..]))
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

#[derive(Deserialize, Debug)]
struct AttestationResponse {
	status: String,
	attestation: Option<String>,
}

#[derive(Error, Debug)]
enum CctpError {
	#[error("Attestation is pending.")]
	AttestationPending,
	#[error("Attestation is disabled.")]
	AttestationDisabled,
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
