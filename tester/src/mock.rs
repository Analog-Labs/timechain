use alloy_primitives::U256;
use alloy_sol_types::{sol, SolValue};
use anyhow::{Context, Result};
use ethers_solc::{artifacts::Source, CompilerInput, Solc};
use rosetta_client::{Blockchain, Wallet};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

#[derive(Clone, Debug)]
pub(crate) struct WalletConfig {
	pub blockchain: Blockchain,
	pub network: String,
	pub url: String,
}

pub(crate) async fn setup_env(config: &WalletConfig, gmp_params: Option<Vec<u8>>) -> (String, u64) {
	fund_wallet(config).await;
	deploy_contract(config, gmp_params).await.unwrap()
}

pub(crate) async fn deploy_contract(
	config: &WalletConfig,
	gmp_params: Option<Vec<u8>>,
) -> Result<(String, u64)> {
	println!("Deploying eth contract");
	let wallet =
		Wallet::new(config.blockchain, &config.network, &config.url, Some("/etc/keyfile".as_ref()))
			.await
			.unwrap();

	let bytes = if let Some(gmp_bytes) = gmp_params {
		let mut gateway_bytes = compile_file("/etc/gateway.sol")?;
		gateway_bytes.extend(gmp_bytes);
		gateway_bytes
	} else {
		compile_file("/etc/test_contract.sol")?
	};

	let tx_hash = wallet.eth_deploy_contract(bytes).await?;
	let tx_receipt = wallet.eth_transaction_receipt(&tx_hash).await?;
	let contract_address = tx_receipt
		.get("contractAddress")
		.and_then(|v| v.as_str().map(str::to_string))
		.ok_or(anyhow::anyhow!("Unable to get contract address"))?;
	let status = wallet.status().await?;

	println!("Deploy contract address {:?} on {:?}", contract_address, status.index);
	Ok((contract_address.to_string(), status.index))
}

pub(crate) async fn fund_wallet(config: &WalletConfig) {
	let wallet =
		Wallet::new(config.blockchain, &config.network, &config.url, Some("/etc/keyfile".as_ref()))
			.await
			.unwrap();
	println!("funding wallet for {:?} {:?}", config.blockchain, wallet.account().address);
	wallet.faucet(1000000000000000000000).await.unwrap();
}

pub(crate) fn drop_node(node_name: String) {
	let output = Command::new("docker")
		.args(["stop", &node_name])
		.output()
		.expect("failed to drop node");
	println!("Dropped node {:?}", output.stderr.is_empty());
}

pub(crate) fn start_node(node_name: String) {
	let output = Command::new("docker")
		.args(["start", &node_name])
		.output()
		.expect("failed to start node");
	println!("Start node {:?}", output.stderr.is_empty());
}

pub(crate) fn restart_node(node_name: String) {
	let output = Command::new("docker")
		.args(["restart", &node_name])
		.output()
		.expect("failed to start node");
	println!("Restart node {:?}", output.stderr.is_empty());
}

pub(crate) fn get_gmp_constructor_params(data: [u8; 33]) -> Vec<u8> {
	let parity_bit = if data[0] % 2 == 0 { 0 } else { 1 };
	let x_coords = hex::encode(&data[1..]);
	sol! {
		#[derive(Debug, PartialEq, Eq)]
		struct TssKey {
			uint8 yParity;
			uint256 xCoord;
		}
	}

	let tss_keys = vec![TssKey {
		yParity: parity_bit,
		xCoord: U256::from_str_radix(&x_coords, 16).unwrap(),
	}];
	tss_keys.abi_encode_params()
}

pub(crate) fn compile_file(path: &str) -> Result<Vec<u8>> {
	let solc = Solc::default();
	let mut sources = BTreeMap::new();
	sources.insert(Path::new(path).into(), Source::read(path).unwrap());
	let input = &CompilerInput::with_sources(sources)[0];
	let output = solc.compile_exact(input)?;
	let file = output.contracts.get(path).unwrap();
	let (key, _) = file.first_key_value().unwrap();
	let contract = file.get(key).unwrap();
	let bytecode = contract
		.evm
		.as_ref()
		.context("evm not found")?
		.bytecode
		.as_ref()
		.context("bytecode not found")?
		.object
		.as_bytes()
		.context("could not convert to bytes")?
		.to_vec();
	Ok(bytecode)
}
