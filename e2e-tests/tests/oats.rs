use alloy::primitives::Bytes;
use alloy::providers::WsConnect;
use alloy::sol_types::SolEvent;
use alloy::{
	network::EthereumWallet, primitives::U256, providers::ProviderBuilder,
	signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use e2e_tests::{Backend, TestEnv};
use futures::stream::StreamExt;
use gmp::Gateway;
use std::sync::Arc;

use tc_cli::MessageTrace;
use time_primitives::MessageId;

mod common;

use common::*;

#[tokio::test]
async fn oats_sender_caller_evm() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	let block = tc.latest_block().await?.0;

	let mut contracts = vec![];
	// Deploy Token + Callee to every network
	for (i, nw_id) in tc.iter().enumerate() {
		let (_, gw) = tc.gateway(nw_id, block).await?;
		let c = env.chain_container(nw_id).unwrap();

		let port = c.get_host_port_ipv4(8545).await.unwrap();
		let ws = WsConnect::new(format!("ws://localhost:{port}"));
		let signer: PrivateKeySigner = ALICE_KEY.parse()?;
		let wallet = EthereumWallet::from(signer.clone());
		let rpc = Arc::new(ProviderBuilder::new().wallet(wallet).connect_ws(ws).await?);

		let token = OATSSenderCaller::deploy(
			rpc.clone(),
			"Omni Token".to_string(),
			"OMNI".to_string(),
			signer.address(),
			U256::from(CAP_AMOUNT),
			a_addr(gw),
		)
		.await?;

		let callee = Callee::deploy(rpc.clone(), *token.address()).await?;
		contracts.push((nw_id, token, callee, GAS_LIMIT_STEP * (i as u64 + 1)));
	}

	common::test_oats_sender_caller(contracts, &tc).await
}

#[tokio::test]
async fn oats_sender_evm() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	let block = tc.latest_block().await?.0;

	let mut contracts = vec![];
	// Deploy Token to every network
	for nw_id in tc.iter() {
		let (_, gw) = tc.gateway(nw_id, block).await?;
		let c = env.chain_container(nw_id).unwrap();

		let port = c.get_host_port_ipv4(8545).await.unwrap();
		let ws = WsConnect::new(format!("ws://localhost:{port}"));
		let signer: PrivateKeySigner = MINTER_KEY.parse()?;
		let wallet = EthereumWallet::from(signer.clone());
		let rpc = Arc::new(ProviderBuilder::new().wallet(wallet).connect_ws(ws).await?);

		let token = OATSSender::deploy(
			rpc,
			"Omni Token".to_string(),
			"OMNI".to_string(),
			signer.address(),
			U256::from(CAP_AMOUNT),
			a_addr(gw),
		)
		.await?;

		contracts.push((nw_id, token));
	}

	common::test_oats_sender(contracts, &tc).await
}

#[tokio::test]
#[ignore]
async fn forever() -> Result<()> {
	let (_env, _tc) = TestEnv::new(Backend::Evm, false).await?;
	tracing::info!("Test env ready. Keeping live indefinitely...");
	#[allow(clippy::empty_loop)]
	loop {}
}
