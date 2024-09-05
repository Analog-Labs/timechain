use crate::sol;
use crate::ufloat::{Float, Rounding, UFloat9x56};
use crate::Connector;
use alloy_primitives::U256;
use alloy_sol_types::{SolCall, SolConstructor};
use anyhow::Result;
use rosetta_client::types::AccountIdentifier;
use rosetta_config_ethereum::{AtBlock, GetTransactionCount};
use rosetta_config_ethereum::{CallResult, SubmitResult};
use std::num::NonZeroU64;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use time_primitives::{
	Address, Gateway, GmpMessage, IConnector, IConnectorAdmin, IConnectorTest, Network, NetworkId,
	TssPublicKey,
};

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
	test: PathBuf,
}

impl Deref for AdminConnector {
	type Target = Connector;

	fn deref(&self) -> &Self::Target {
		&self.connector
	}
}

impl AdminConnector {
	async fn nonce(&self) -> Result<u64> {
		let admin = alloy_primitives::Address::from_str(self.connector.account())?;
		self.wallet()
			.query(GetTransactionCount {
				address: admin.into_array().into(),
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
		let result = self
			.connector
			.wallet()
			.eth_send_call(sol::addr(contract).into(), call.abi_encode(), amount, nonce, None)
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
		let block: AtBlock = if let Some(block) = block { block.into() } else { AtBlock::Latest };
		let result = self
			.connector
			.wallet()
			.eth_view_call(sol::addr(contract).into(), call.abi_encode(), block)
			.await?;
		let CallResult::Success(result) = result else { anyhow::bail!("{:?}", result) };
		Ok(T::abi_decode_returns(&result, true)?)
	}

	async fn deploy_contract(
		&self,
		path: &Path,
		constructor: impl SolConstructor,
	) -> Result<(Address, u64)> {
		let mut contract = compile_file(path)?;
		contract.extend(constructor.abi_encode());
		let tx_hash = self.wallet.eth_deploy_contract(contract).await?.tx_hash().0;
		let tx_receipt = self.wallet.eth_transaction_receipt(tx_hash).await?.unwrap();
		let contract_address = tx_receipt.contract_address.unwrap();
		let block_number = tx_receipt.block_number.unwrap();
		let mut addr = [0; 32];
		addr[12..].copy_from_slice(&contract_address.0);
		Ok((addr, block_number))
	}
}

#[async_trait::async_trait]
impl IConnectorAdmin for AdminConnector {
	type Connector = Connector;

	fn new(connector: Self::Connector, gateway: PathBuf, proxy: PathBuf, test: PathBuf) -> Self {
		Self {
			connector,
			gateway,
			proxy,
			test,
		}
	}

	async fn deploy_gateway(&self) -> Result<(Gateway, u64)> {
		let admin = alloy_primitives::Address::from_str(self.connector.account())?;

		// get deployer's nonce
		let nonce = self.nonce().await?;

		// compute the proxy address
		let proxy_addr = admin.create(nonce + 1);

		// deploy gateway implementation
		println!("deploying implementation contract...");
		let call = sol::IGateway::constructorCall {
			networkId: self.network_id,
			proxy: proxy_addr,
		};
		let (gateway_addr, _) = self.deploy_contract(&self.gateway, call).await?;

		// deploy and initialize gateway proxy
		println!("deploying proxy contract...");
		// Build the Gateway initializer
		let initializer = sol::Gateway::initializeCall {
			admin: admin.into_array().into(),
			keys: vec![],
			networks: vec![],
		}
		.abi_encode();

		// Deploy the proxy contract
		let call = sol::GatewayProxy::constructorCall {
			implementation: sol::addr(gateway_addr),
			initializer: initializer.into(),
		};
		let (actual_addr, block_number) = self.deploy_contract(&self.proxy, call).await?;

		// Check if the proxy address match the expect address
		if proxy_addr != sol::addr(actual_addr) {
			anyhow::bail!("Proxy address mismatch, expect {proxy_addr:?}, got {actual_addr:?}");
		}
		Ok((actual_addr.into(), block_number))
	}

	async fn redeploy_gateway(&self, gateway: Gateway) -> Result<()> {
		let call = sol::IGateway::constructorCall {
			networkId: self.network_id,
			proxy: sol::addr(gateway),
		};
		let (gateway_addr, _) = self.deploy_contract(&self.gateway, call).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: sol::addr(gateway_addr).into(),
		}
		.abi_encode();
		println!("call data for gateway update: {:?}", hex::encode(&call));
		let result = self
			.wallet()
			.eth_send_call(sol::addr(gateway).into(), call, 0, None, None)
			.await?;
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

	async fn admin(&self, _gateway: Gateway) -> Result<Address> {
		todo!()
	}

	async fn set_admin(&self, gateway: Gateway, new_admin: Address) -> Result<()> {
		let call = sol::Gateway::setAdminCall { newAdmin: sol::addr(new_admin) }.abi_encode();
		println!("call data for set admin: {:?}", hex::encode(&call));
		let result = self
			.wallet()
			.eth_send_call(sol::addr(gateway).into(), call, 0, None, None)
			.await?;
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

	async fn shards(&self, _gateway: Gateway) -> Result<Vec<TssPublicKey>> {
		todo!()
	}

	async fn set_shards(&self, gateway: Gateway, keys: &[TssPublicKey]) -> Result<()> {
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
		let result = self
			.wallet()
			.eth_send_call(sol::addr(gateway).into(), call, 0, None, None)
			.await?;
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

	async fn networks(&self) -> Result<Vec<Network>> {
		todo!()
	}

	async fn set_network(&self, gateway: Gateway, network: Network) -> Result<()> {
		let float =
			UFloat9x56::rational_to_float(network.relative_gas_price, Rounding::NearestTiesEven)
				.map_err(|e| anyhow::anyhow!("{:?}", e))?;
		let call = sol::Gateway::setNetworkInfoCall {
			info: sol::UpdateNetworkInfo {
				networkId: network.network_id,
				domainSeparator: [0; 32].into(),
				gasLimit: network.gas_limit,
				relativeGasPrice: float,
				baseFee: network.base_fee,
				mortality: u64::MAX,
			},
		}
		.abi_encode();
		self.wallet()
			.eth_send_call(sol::addr(gateway).into(), call, 0, None, None)
			.await?;
		Ok(())
	}
}

#[async_trait::async_trait]
impl IConnectorTest for AdminConnector {
	async fn faucet(&self) -> Result<()> {
		// TODO: Calculate the gas_limit necessary to execute the test, then replace this
		// by: gas_limit * gas_price, where gas_price changes depending on the network
		let balance = match self.network_id() {
			6 => 10u128.pow(25), // astar
			3 => 10u128.pow(29), // ethereum
			network_id => {
				tracing::info!("network {network_id} doesn't support faucet");
				return Ok(());
			},
		};
		self.wallet().faucet(balance).await?;
		Ok(())
	}

	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		self.connector
			.wallet()
			.transfer(&AccountIdentifier::new(sol::addr(address).to_string()), amount, None, None)
			.await?;
		Ok(())
	}

	async fn deploy_test(&self, gateway: Gateway) -> Result<(Address, u64)> {
		self.deploy_contract(
			&self.test,
			sol::GmpTester::constructorCall { gateway: sol::addr(gateway) },
		)
		.await
	}

	async fn estimate_message_cost(
		&self,
		gateway: Gateway,
		dest: NetworkId,
		msg_size: usize,
	) -> Result<u128> {
		let msg_size = U256::from_str_radix(&msg_size.to_string(), 16).unwrap();
		let result = self
			.wallet()
			.eth_send_call(
				sol::addr(gateway).into(),
				sol::Gateway::estimateMessageCostCall {
					networkid: dest,
					messageSize: msg_size,
					gasLimit: U256::from(100_000),
				}
				.abi_encode(),
				0,
				None,
				None,
			)
			.await?;
		let SubmitResult::Executed { result, .. } = result else { anyhow::bail!("{:?}", result) };
		let CallResult::Success(data) = result else {
			anyhow::bail!("failed parsing {:?}", result)
		};
		let msg_cost: u128 =
			sol::Gateway::estimateMessageCostCall::abi_decode_returns(&data, true)?
				._0
				.try_into()
				.unwrap();
		Ok(msg_cost)
	}

	async fn send_message(&self, _addr: Address, _msg: GmpMessage) -> Result<()> {
		todo!()
	}

	async fn recv_messages(
		&self,
		_addr: Address,
		_from_block: Option<NonZeroU64>,
		_to_block: u64,
	) -> Result<Vec<GmpMessage>> {
		todo!()
	}
}
