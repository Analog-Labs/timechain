use alloy::primitives::U256;
use anyhow::Result;
use e2e_tests::{Backend, TestEnv, ANVIL_PORT};

mod common;

use common::*;

#[tokio::test]
async fn oats_sender_caller_evm() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	let block = tc.latest_block().await?.0;

	let mut contracts = vec![];
	// Deploy Token + Callee to every network
	for (i, nw_id) in tc.iter().enumerate() {
		let port = env.chain_container(nw_id)?.get_host_port_ipv4(ANVIL_PORT).await?;
		let rpc = common::build_rpc(MINTER_KEY, port).await?;

		let (_, gw) = tc.gateway(nw_id, block).await?;
		let token = OATSSenderCaller::deploy(
			rpc.clone(),
			"Omni Token".to_string(),
			"OMNI".to_string(),
			MINTER,
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
		let port = env.chain_container(nw_id)?.get_host_port_ipv4(ANVIL_PORT).await?;
		let rpc = common::build_rpc(MINTER_KEY, port).await?;
		let (_, gw) = tc.gateway(nw_id, block).await?;

		let token = OATSSender::deploy(
			rpc,
			"Omni Token".to_string(),
			"OMNI".to_string(),
			MINTER,
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
