use alloy::{
	dyn_abi::DynSolValue,
	network::{AnyNetwork, AnyReceiptEnvelope, EthereumWallet, TransactionBuilder},
	primitives::U256,
	providers::{
		fillers::{
			BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
			WalletFiller,
		},
		Provider, ProviderBuilder, RootProvider, WsConnect,
	},
	rpc::types::{Log, TransactionReceipt, TransactionRequest},
	serde::WithOtherFields,
	signers::local::{coins_bip39::English, MnemonicBuilder},
	sol,
	sol_types::{SolCall, SolConstructor, SolEvent, SolValue},
};
use anyhow::{Context, Result};
use scale_codec::Decode;
use serde::Deserialize;
use std::{
	collections::HashMap,
	sync::Arc,
	time::{Duration, SystemTime, UNIX_EPOCH},
};
use time_primitives::{Address32, BlockHash, MessageId, NetworkId};

use crate::{config::SwapPrerequisites, Mnemonics, Tc};
type Address20 = alloy::primitives::Address;
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

// circle message v1 bytecode doesnt need to be changed so keeping it as const.
pub const CIRCLE_MESSAGE_LIB_BYTECODE: &str = "610106610034600b8282823980515f1a607314602857634e487b7160e01b5f525f60045260245ffd5b305f52607381538281f3fe7300000000000000000000000000000000000000003014608060405260043610603c575f3560e01c80635ced058e14604057806382c947b714606b575b5f80fd5b604e604b366004608f565b90565b6040516001600160a01b0390911681526020015b60405180910390f35b6082607636600460a5565b6001600160a01b031690565b6040519081526020016062565b5f60208284031215609e575f80fd5b5035919050565b5f6020828403121560b4575f80fd5b81356001600160a01b038116811460c9575f80fd5b939250505056fea2646970667358221220c4a4b6631fa0b2b666a9bfb1b09eddaaf35ed46e87ed97746419c81c07fd02cb64736f6c63430008190033";

pub struct EthWallet {
	pub provider: Arc<CProvider>,
}

impl EthWallet {
	async fn new(url: String) -> Result<Self> {
		let mnemonic = Mnemonics::from_env();
		let signer = MnemonicBuilder::<English>::default()
			.phrase(mnemonic.target_mnemonic)
			.index(0)?
			.build()?;
		let ws = WsConnect::new(url.clone())
			.with_max_retries(1200)
			.with_retry_interval(Duration::from_secs(3));

		let provider = Arc::new(
			ProviderBuilder::new()
				.network::<AnyNetwork>()
				.wallet(signer.clone())
				.connect_ws(ws)
				.await?,
		);

		Ok(Self { provider })
	}
	async fn evm_send<C: SolCall>(
		&self,
		to: Address32,
		call: C,
		value: u128,
	) -> Result<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>> {
		let tx = TransactionRequest::default()
			.with_to(a_addr(to))
			.with_chain_id(self.provider.get_chain_id().await?)
			.with_call(&call)
			.with_value(U256::from(value));
		let pending_tx = self.provider.send_transaction(WithOtherFields::new(tx)).await?;
		tracing::info!("Tx sent: {:?}", pending_tx.tx_hash());

		Ok(pending_tx.get_receipt().await?)
	}
}

impl Tc {
	pub async fn deploy_zenswap(
		&self,
		network: NetworkId,
		block_hash: BlockHash,
	) -> Result<(Address32, Address32)> {
		let src_url = self
			.config
			.networks()
			.get(&network)
			.ok_or(anyhow::anyhow!("Config does not contain network: {:?}", network))?
			.url
			.clone();
		let eth_wallet = EthWallet::new(src_url).await?;
		let backend = self.config.backend(network)?;
		let network_config = self.config.network(network)?;
		let (Some(zenswap_code), Some(zenswap_plugin_code), Some(helper_contracts)) =
			(backend.zenswap, backend.zenswap_plugin, network_config.zenswap.clone())
		else {
			anyhow::bail!("Zenswap not supported on {network}");
		};

		let (_, gateway) = self.gateway(network, block_hash).await?;

		let helper_contracts =
			helper_contracts.to_address32(network, |net, addr| self.parse_address(net, addr))?;
		let universal_router = a_addr(helper_contracts.universal_router);
		let transmitter = a_addr(helper_contracts.msg_transmitter);
		let permit2 = a_addr(helper_contracts.permit2);
		let messenger = a_addr(helper_contracts.token_messenger);
		let usdc = a_addr(helper_contracts.usdc);

		let plugin_initializer = ZenSwapGmpPlugin::initializeCall {
			_gmpGateway: a_addr(gateway),
			_cctpMessenger: messenger,
			_cctpReceiver: transmitter,
			_usdc: usdc,
			_fee: U256::from_be_bytes([0u8; 32]),
		};

		let zenswap_contructor = ZenSwap::constructorCall {
			_universalRouter: universal_router,
			_permit2: permit2,
		};

		let mut zenswap_bytecode = extract_bytecode(&zenswap_code, Default::default())?;
		zenswap_bytecode.extend(zenswap_contructor.abi_encode());

		// Zenswap deployment
		let tx = TransactionRequest::default().with_deploy_code(zenswap_bytecode);
		let receipt = eth_wallet
			.provider
			.send_transaction(WithOtherFields::new(tx))
			.await?
			.get_receipt()
			.await?;
		let zenswap_addr = receipt
			.contract_address
			.ok_or(anyhow::anyhow!("Unable to get contract address"))?;

		// Message lib deployment
		let msg_lib_code = hex::decode(CIRCLE_MESSAGE_LIB_BYTECODE)?;
		let tx = TransactionRequest::default().with_deploy_code(msg_lib_code);
		let receipt = eth_wallet
			.provider
			.send_transaction(WithOtherFields::new(tx))
			.await?
			.get_receipt()
			.await?;
		let lib_addr = receipt
			.contract_address
			.ok_or(anyhow::anyhow!("Unable to get message library address"))?;

		// ZenswapPlugin deployment
		let mut replacement_keys = HashMap::new();
		replacement_keys.insert("__$2e72248e36cbd9e27bfc8c16586a2f5547$__", hex::encode(lib_addr));
		let plugin_bytecode = extract_bytecode(&zenswap_plugin_code, replacement_keys)?;
		let tx = TransactionRequest::default().with_deploy_code(plugin_bytecode);
		let receipt = eth_wallet
			.provider
			.send_transaction(WithOtherFields::new(tx))
			.await?
			.get_receipt()
			.await?;
		let plugin_address = receipt
			.contract_address
			.ok_or(anyhow::anyhow!("Unable to get plugin address"))?;

		//initialize the plugin
		eth_wallet.evm_send(t_addr(plugin_address), plugin_initializer, 0).await?;
		tracing::info!("zenswap addr: {}", hex::encode(zenswap_addr));
		tracing::info!("zenswap plugin addr: {}", hex::encode(plugin_address));
		Ok((t_addr(zenswap_addr), t_addr(plugin_address)))
		// Ok(tester)
	}

	#[allow(clippy::too_many_arguments)]
	pub async fn send_swap(
		&self,
		src: NetworkId,
		dest: NetworkId,
		src_zen: Address32,
		src_plugin: Address32,
		dest_zen: Address32,
		dest_plugin: Address32,
		block_hash: BlockHash,
	) -> Result<MessageId> {
		let src_url = self
			.config
			.networks()
			.get(&src)
			.ok_or(anyhow::anyhow!("Config does not contain network: {:?}", src))?
			.url
			.clone();
		let eth_wallet = EthWallet::new(src_url).await?;
		let (src_contracts, dest_contracts) = self.get_swap_contracts(src, dest)?;
		let src_contracts =
			src_contracts.to_address32(src, |net, addr| self.parse_address(net, addr))?;
		let dest_contracts =
			dest_contracts.to_address32(dest, |net, addr| self.parse_address(net, addr))?;

		let dest_chain_name =
			self.runtime.network_name(dest, block_hash).await?.context("invalid network")?;
		let dest_chain_name =
			String::decode(&mut dest_chain_name.0.to_vec().as_slice()).unwrap_or_default();

		let sender = self.address(Some(src))?;
		let src_usdc = a_addr(src_contracts.usdc);
		let dest_usdc = a_addr(dest_contracts.usdc);

		let domain_id = chain_to_domain_id(&dest_chain_name)?;
		let params = ZenSwapGmpPlugin::PluginParams {
			destPlugin: a_addr(dest_plugin),
			recipient: a_addr(dest_zen),
			fallbackRecipient: a_addr(sender),
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
			DynSolValue::Bytes(src_path_encoded),
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
			DynSolValue::Address(a_addr(src_zen)),
			DynSolValue::Uint(U256::from(1), 256),
		])
		.abi_encode();

		let src_swap_params = ZenSwap::SwapParams {
			tokenIn: Address20::ZERO,
			tokenOut: src_usdc,
			deadline: U256::from(deadline),
			commands: hex::decode("0b0004").unwrap().into(),
			inputs: vec![wrap_eth.into(), src_swap_data.into(), sweep_data.into()],
		};
		/////////////

		// Dest side, swapping setup.
		let dest_swap_params = ZenSwap::SwapParams {
			tokenIn: dest_usdc,
			tokenOut: dest_usdc,
			deadline: U256::from(deadline),
			commands: vec![].into(),
			inputs: vec![],
		};
		/////////

		let swap_call = ZenSwap::swapSendCall {
			pluginParams: params.abi_encode().into(),
			sourceParams: src_swap_params,
			destParams: dest_swap_params,
			recipient: a_addr(sender),
			plugin: a_addr(src_plugin),
			amountIn: amount,
		};

		let connector = self.connector(src)?;
		let gas_cost = connector
			.estimate_message_cost(src_plugin, dest, 1_000_000, swap_call.abi_encode())
			.await?;
		let receipt = eth_wallet.evm_send(src_zen, swap_call, gas_cost + amount_u128).await?;
		receipt
			.inner
			.inner
			.logs()
			.iter()
			.filter(|e| e.topics().contains(&Gateway::GmpCreated::SIGNATURE_HASH))
			.filter_map(|e| Gateway::GmpCreated::decode_log_data(e.data()).ok())
			.map(|e| e.id.into())
			.next()
			.ok_or(anyhow::anyhow!("Failed to send message"))
	}

	pub fn get_swap_contracts(
		&self,
		src: NetworkId,
		dest: NetworkId,
	) -> Result<(SwapPrerequisites, SwapPrerequisites)> {
		let src_backend = self.config.backend(src)?;
		let dest_backend = self.config.backend(dest)?;
		let src_config = self.config.network(src)?;
		let dest_config = self.config.network(dest)?;
		let (Some(_), Some(_), Some(_), Some(_), Some(src_contracts), Some(dest_contracts)) = (
			src_backend.zenswap,
			src_backend.zenswap_plugin,
			dest_backend.zenswap,
			dest_backend.zenswap_plugin,
			src_config.zenswap.clone(),
			dest_config.zenswap.clone(),
		) else {
			anyhow::bail!("Swap not supported between {src} {dest}");
		};
		Ok((src_contracts, dest_contracts))
	}
}

sol! {
	contract ZenSwap {
		constructor(address _universalRouter, address _permit2) UniswapWrapper(_universalRouter, _permit2);
		struct SwapParams {
			address tokenIn;
			address tokenOut;
			uint256 deadline;
			bytes commands;
			bytes[] inputs;
		}

		function swapSend(
			bytes calldata pluginParams,
			SwapParams calldata sourceParams,
			SwapParams calldata destParams,
			address payable recipient,
			address plugin,
			uint256 amountIn
		) external payable;
	}

	contract ZenSwapGmpPlugin {
		function initialize(
			address _gmpGateway,
			address _cctpMessenger,
			address _cctpReceiver,
			address _usdc,
			uint _fee
		) public initializer;

		struct PluginParams {
			// Plugin address on destination chain
			address destPlugin;
			// Extra data recipient (ZenSwap contract)
			address recipient;
			// USDC recipient in case of onReceived fail
			address fallbackRecipient;
			// CCTP destination domain
			uint32 cctpDestinationDomain;
			// GMP destination network id
			uint16 gmpDestNetwork;
			// GMP gas limit
			uint64 gmpGasLimit;
		}
	}

	contract Gateway{
		event GmpCreated(
			bytes32 indexed id,
			bytes32 indexed source,
			address indexed destinationAddress,
			uint16 destinationNetwork,
			uint64 executionGasLimit,
			uint64 gasCost,
			uint64 nonce,
			bytes data
		);
	}
}

fn chain_to_domain_id(chain_name: &str) -> Result<u32> {
	match chain_name {
		"ethereum sepolia" => Ok(0),
		"arbitrum sepolia" => Ok(3),
		_ => anyhow::bail!("Unsupported chain name for cctp"),
	}
}

fn extract_bytecode(json_abi: &[u8], link_addresses: HashMap<&str, String>) -> Result<Vec<u8>> {
	let contract_abi: Contract = serde_json::from_slice(json_abi)?;
	let mut bytecode_str = match contract_abi.bytecode {
		Bytecode::Object { object } => object,
		Bytecode::Code(code) => code,
	};
	for (key, val) in link_addresses.iter() {
		bytecode_str = bytecode_str.replace(key, val);
	}
	let bytecode_str = bytecode_str.replace("0x", "");
	hex::decode(bytecode_str).with_context(|| "Failed to get contract bytecode")
}

fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}

fn t_addr(address: Address20) -> Address32 {
	address.into_word().into()
}

#[derive(Deserialize)]
struct Contract {
	bytecode: Bytecode,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Bytecode {
	Object { object: String },
	Code(String),
}
