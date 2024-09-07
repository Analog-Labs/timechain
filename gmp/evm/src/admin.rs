use crate::sol;
use crate::ufloat::{Float, Rounding, UFloat9x56};
use crate::Address;
use alloy_primitives::U256;
use alloy_sol_types::{SolCall, SolConstructor};
use anyhow::Result;
use rosetta_client::types::AccountIdentifier;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{AtBlock, GetTransactionCount};
use rosetta_config_ethereum::{CallResult, SubmitResult};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use time_primitives::{IConnectorAdmin, Msg, Network, NetworkId, TssPublicKey};

pub use rosetta_config_ethereum::TransactionReceipt;

type TssKeyR = <sol::TssKey as alloy_sol_types::SolType>::RustType;

fn compile_file(path: &Path) -> Result<Vec<u8>> {
	let abi = std::fs::read_to_string(path)?;
	let json_abi: serde_json::Value = serde_json::from_str(&abi)?;
	Ok(hex::decode(json_abi["bytecode"]["object"].as_str().unwrap().replace("0x", ""))?)
}

pub struct AdminConnector {
	wallet: Arc<Wallet>,
	network_id: NetworkId,
	gateway: PathBuf,
	proxy: PathBuf,
	tester: PathBuf,
}

impl AdminConnector {
	async fn nonce(&self, address: Address) -> Result<u64> {
		self.wallet
			.query(GetTransactionCount {
				address: address.0.into(),
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
			.wallet
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

	async fn evm_view<T: SolCall>(
		&self,
		contract: Address,
		call: T,
		block: Option<u64>,
	) -> Result<T::Return> {
		let block: AtBlock = if let Some(block) = block { block.into() } else { AtBlock::Latest };
		let result = self.wallet.eth_view_call(contract.into(), call.abi_encode(), block).await?;
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
		Ok((contract_address.0.into(), block_number))
	}
}

#[async_trait::async_trait]
impl IConnectorAdmin for AdminConnector {
	type Address = Address;

	async fn new(
		network_id: NetworkId,
		blockchain: &str,
		network: &str,
		url: &str,
		keyfile: &Path,
		gateway: PathBuf,
		proxy: PathBuf,
		tester: PathBuf,
	) -> Result<Self> {
		let wallet =
			Arc::new(Wallet::new(blockchain.parse()?, network, url, Some(keyfile), None).await?);
		Ok(Self {
			wallet,
			network_id,
			gateway,
			proxy,
			tester,
		})
	}

	fn network_id(&self) -> NetworkId {
		self.network_id
	}

	fn address(&self) -> Self::Address {
		Address::from_str(&self.wallet.account().address).unwrap()
	}

	/// Checks if wallet have enough balance
	async fn balance(&self, _addr: Address) -> Result<u128> {
		//self.wallet.balance(addr).await
		todo!()
	}

	async fn deploy_gateway(&self) -> Result<(Address, u64)> {
		let admin = self.address();

		// get deployer's nonce
		let nonce = self.nonce(admin).await?;

		// compute the proxy address
		let proxy_addr = Address::from(alloy_primitives::Address::from(admin).create(nonce + 1));

		// deploy gateway
		let call = sol::IGateway::constructorCall {
			networkId: self.network_id,
			proxy: proxy_addr.into(),
		};
		let (gateway_addr, _) = self.deploy_contract(&self.gateway, call).await?;

		// deploy the proxy contract
		let initializer = sol::Gateway::initializeCall {
			admin: admin.into(),
			keys: vec![],
			networks: vec![],
		}
		.abi_encode();
		let call = sol::GatewayProxy::constructorCall {
			implementation: gateway_addr.into(),
			initializer: initializer.into(),
		};
		let (actual_addr, block_number) = self.deploy_contract(&self.proxy, call).await?;

		// Check if the proxy address match the expect address
		if proxy_addr != actual_addr {
			anyhow::bail!("Proxy address mismatch, expect {proxy_addr:?}, got {actual_addr:?}");
		}
		Ok((actual_addr, block_number))
	}

	async fn redeploy_gateway(&self, gateway: Address) -> Result<()> {
		let call = sol::IGateway::constructorCall {
			networkId: self.network_id,
			proxy: gateway.into(),
		};
		let (gateway_addr, _) = self.deploy_contract(&self.gateway, call).await?;
		let call = sol::Gateway::upgradeCall {
			newImplementation: gateway_addr.into(),
		};
		self.evm_call(gateway.into(), call, 0, None).await?;
		Ok(())
	}

	async fn admin(&self, gateway: Address) -> Result<Address> {
		let result = self.evm_view(gateway, sol::Gateway::adminCall {}, None).await?;
		Ok(result._0.into())
	}

	async fn set_admin(&self, gateway: Address, new_admin: Address) -> Result<()> {
		let call = sol::Gateway::setAdminCall { newAdmin: new_admin.into() };
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}

	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>> {
		let result = self.evm_view(gateway, sol::Gateway::shardsCall {}, None).await?;
		let keys = result
			._0
			.into_iter()
			.map(|key| {
				let mut tss_key = [0; 33];
				tss_key[1..].copy_from_slice(&key.xCoord.to_be_bytes::<8>());
				tss_key[0] = key.yParity;
				tss_key
			})
			.collect();
		Ok(keys)
	}

	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()> {
		let tss_keys = keys
			.into_iter()
			.map(|key| TssKeyR {
				yParity: if key[0] % 2 == 0 { 0 } else { 1 },
				xCoord: U256::from_be_bytes(<[u8; 32]>::try_from(&key[1..]).unwrap()),
			})
			.collect::<Vec<_>>();
		let call = sol::Gateway::sudoAddShardsCall { shards: tss_keys };
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}

	async fn networks(&self, gateway: Address) -> Result<Vec<Network>> {
		let result = self.evm_view(gateway, sol::Gateway::networksCall {}, None).await?;
		let networks = result
			._0
			.into_iter()
			.map(|net| Network {
				network_id: net.networkId,
				gateway: todo!(),
				relative_gas_price: todo!(),
				gas_limit: net.gasLimit,
				base_fee: net.baseFee,
			})
			.collect();
		Ok(networks)
	}

	async fn set_network(&self, gateway: Address, network: Network) -> Result<()> {
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
		};
		self.evm_call(gateway, call, 0, None).await?;
		Ok(())
	}

	async fn faucet(&self) -> Result<()> {
		let balance = match self.network_id() {
			6 => 10u128.pow(25), // astar
			3 => 10u128.pow(29), // ethereum
			network_id => {
				tracing::info!("network {network_id} doesn't support faucet");
				return Ok(());
			},
		};
		self.wallet.faucet(balance).await?;
		Ok(())
	}

	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		self.wallet
			.transfer(&AccountIdentifier::new(address.to_string()), amount, None, None)
			.await?;
		Ok(())
	}

	async fn deploy_test(&self, gateway: Address) -> Result<(Address, u64)> {
		self.deploy_contract(
			&self.tester,
			sol::GmpTester::constructorCall { gateway: gateway.into() },
		)
		.await
	}

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

	async fn send_message(&self, _addr: Address, _msg: Msg) -> Result<()> {
		todo!()
	}

	async fn recv_messages(&self, _addr: Address, _blocks: Range<u64>) -> Result<Vec<Msg>> {
		todo!()
	}
}
