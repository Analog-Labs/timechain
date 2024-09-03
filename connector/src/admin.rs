use crate::gateway::IGateway;
use crate::sol;
use crate::ufloat::{Float, Rounding, UFloat9x56};
use crate::Connector;
use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolCall, SolConstructor};
use anyhow::Result;
use num_bigint::BigUint;
use rosetta_client::types::AccountIdentifier;
use rosetta_config_ethereum::{AtBlock, GetTransactionCount};
use rosetta_config_ethereum::{CallResult, SubmitResult};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use time_primitives::{Gateway, NetworkId, TssPublicKey};

pub use rosetta_config_ethereum::TransactionReceipt;

type TssKeyR = <sol::TssKey as alloy_sol_types::SolType>::RustType;

fn compile_file(path: &Path) -> Result<Vec<u8>> {
	let abi = std::fs::read_to_string(path)?;
	let json_abi: serde_json::Value = serde_json::from_str(&abi)?;
	Ok(hex::decode(json_abi["bytecode"]["object"].as_str().unwrap().replace("0x", ""))?)
}

pub struct AdminConnector {
	connector: Connector,
	gateway: PathBuf,
	proxy: PathBuf,
}

impl Deref for AdminConnector {
	type Target = Connector;

	fn deref(&self) -> &Self::Target {
		&self.connector
	}
}

impl AdminConnector {
	pub fn new(connector: Connector, gateway: PathBuf, proxy: PathBuf) -> Self {
		Self { connector, gateway, proxy }
	}

	pub async fn nonce(&self) -> Result<u64> {
		let admin = Address::from_str(self.connector.account())?;
		self.wallet()
			.query(GetTransactionCount {
				address: admin.into_array().into(),
				block: AtBlock::Latest,
			})
			.await
	}

	pub async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		self.connector
			.wallet()
			.transfer(&AccountIdentifier::new(address.to_string()), amount, None, None)
			.await?;
		Ok(())
	}

	pub async fn evm_call<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		amount: u128,
		nonce: Option<u64>,
	) -> Result<(T::Return, TransactionReceipt, [u8; 32])> {
		let result = self
			.connector
			.wallet()
			.eth_send_call(contract.into(), call.abi_encode(), amount, nonce, None)
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

	pub async fn evm_view<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		block: Option<u64>,
	) -> Result<T::Return> {
		let block: AtBlock = if let Some(block) = block { block.into() } else { AtBlock::Latest };
		let result = self
			.connector
			.wallet()
			.eth_view_call(contract.into(), call.abi_encode(), block)
			.await?;
		let CallResult::Success(result) = result else { anyhow::bail!("{:?}", result) };
		Ok(T::abi_decode_returns(&result, true)?)
	}

	pub async fn faucet(&self) {
		// TODO: Calculate the gas_limit necessary to execute the test, then replace this
		// by: gas_limit * gas_price, where gas_price changes depending on the network
		let balance = match self.network_id() {
			6 => 10u128.pow(25), // astar
			3 => 10u128.pow(29), // ethereum
			network_id => {
				println!("network id: {network_id} not compatible for faucet");
				return;
			},
		};
		if let Err(err) = self.wallet().faucet(balance).await {
			println!("Error occured while funding wallet {:?}", err);
		}
	}

	pub async fn deploy_contract(
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

	pub async fn deploy_gateway(&self, keys: &[TssPublicKey]) -> Result<(Gateway, u64)> {
		let admin = Address::from_str(self.connector.account())?;

		// get deployer's nonce
		let nonce = self.nonce().await?;

		// compute the proxy address
		let proxy_addr = admin.create(nonce + 1);

		// deploy gateway implementation
		println!("deploying implementation contract...");
		let call = IGateway::constructorCall {
			networkId: self.network_id,
			proxy: proxy_addr,
		};
		let (gateway_addr, _) = self.deploy_contract(&self.gateway, call).await?;

		// deploy and initialize gateway proxy
		println!("deploying proxy contract...");
		// Build the Gateway initializer
		let tss_keys: Vec<TssKeyR> = keys
			.iter()
			.map(|key| {
				let parity_bit = if key[0] % 2 == 0 { 0 } else { 1 };
				let x_coords = hex::encode(&key[1..]);
				TssKeyR {
					yParity: parity_bit,
					xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
				}
			})
			.collect();
		let initializer = sol::Gateway::initializeCall {
			admin: admin.into_array().into(),
			keys: tss_keys,
			networks: vec![],
		}
		.abi_encode();

		// Deploy the proxy contract
		let call = sol::GatewayProxy::constructorCall {
			implementation: gateway_addr.into(),
			initializer: initializer.into(),
		};
		let (actual_addr, block_number) = self.deploy_contract(&self.proxy, call).await?;

		// Check if the proxy address match the expect address
		let actual_addr: Address = actual_addr.into();
		if actual_addr != proxy_addr.into_array() {
			anyhow::bail!("Proxy address mismatch, expect {proxy_addr:?}, got {actual_addr:?}");
		}
		Ok((actual_addr.into_array(), block_number))
	}

	pub async fn redeploy_gateway(&self, gateway: Gateway) -> Result<()> {
		let call = IGateway::constructorCall {
			networkId: self.network_id,
			proxy: gateway.into(),
		};
		let (gateway_addr, _) = self.deploy_contract(&self.gateway, call).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: gateway_addr.into(),
		}
		.abi_encode();
		println!("call data for gateway update: {:?}", hex::encode(&call));
		let result = self.wallet().eth_send_call(gateway, call, 0, None, None).await?;
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

	pub async fn sudo_set_admin(&self, gateway: Gateway, new_admin: Address) -> Result<()> {
		let call = sol::Gateway::setAdminCall { newAdmin: new_admin }.abi_encode();
		println!("call data for set admin: {:?}", hex::encode(&call));
		let result = self.wallet().eth_send_call(gateway, call, 0, None, None).await?;
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

	pub async fn sudo_register_shards(
		&self,
		gateway: Gateway,
		keys: &[TssPublicKey],
	) -> Result<()> {
		let tss_keys = keys
			.into_iter()
			.map(|key| {
				let parity_bit = if key[0] % 2 == 0 { 0 } else { 1 };
				let x_coords = hex::encode(&key[1..]);
				TssKeyR {
					yParity: parity_bit,
					xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
				}
			})
			.collect::<Vec<_>>();
		let call = sol::Gateway::sudoAddShardsCall { shards: tss_keys }.abi_encode();
		println!("call data for add shards: {:?}", hex::encode(&call));
		let result = self.wallet().eth_send_call(gateway, call, 0, None, None).await?;
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

	pub async fn sudo_set_network_info(
		&self,
		gateway: Gateway,
		network_id: NetworkId,
		dest_gateway: Gateway,
		numerator: BigUint,
		denominator: BigUint,
		gas_limit: u64,
		base_fee: u128,
	) -> Result<()> {
		let float =
			UFloat9x56::rational_to_float(numerator, denominator, Rounding::NearestTiesEven)
				.map_err(|e| anyhow::anyhow!("{:?}", e))?;
		let call = sol::Gateway::setNetworkInfoCall {
			info: sol::UpdateNetworkInfo {
				networkId: network_id,
				domainSeparator: [0; 32].into(),
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
}
