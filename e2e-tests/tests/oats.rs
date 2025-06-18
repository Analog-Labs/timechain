use alloy::primitives::Bytes;
use alloy::providers::{WsConnect};
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

	const GAS_LIMIT_STEP: u64 = 50_000;

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
	// Set OMNI token networks
	for (nw, token, _, _) in contracts.iter() {
		for (n, t, _, _) in contracts.iter().filter(|(n, _, _, _)| n.ne(nw)) {
			token.set_network(*n, *t.address()).send().await?.get_receipt().await?;
		}
	}
	// Check initial balances
	let mut alice_balances = vec![];
	for (_nw, token, callee, _) in contracts.iter() {
		let alice_bal = token.balanceOf(ALICE).call().await?;
		let bob_bal = token.balanceOf(MINTER).call().await?;
		// On every chain, ALICE has some OMNI tokens, and MINTER has none.
		assert_ne!(alice_bal, U256::ZERO);
		assert_eq!(bob_bal, U256::ZERO);
		alice_balances.push(alice_bal);
		// Callee total is unitialized hence ZERO
		assert_eq!(callee.total().call().await?, U256::ZERO);
	}
	// Transfer tokens from every network to next network, and call callee, ring way
	let mut msgs = vec![];
	let mut ring = contracts.iter().cycle().take(contracts.len() + 1).peekable();
	while let Some((nw, token, callee, gas_limit)) = ring.next() {
		if let Some((nw2, _, _, _)) = ring.peek() {
			let gmp_fee = token.cost(*nw2, *gas_limit, Bytes::new()).call().await?;
			let receipt = token
				.sendAndCall(
					*nw2,
					MINTER,
					U256::from(TRANSFER_AMOUNT),
					*gas_limit,
					*callee.address(),
					Bytes::new(),
				)
				.value(gmp_fee)
				.send()
				.await?
				.get_receipt()
				.await?;

			let msg_id: MessageId = receipt
				.inner
				.logs()
				.iter()
				.filter(|e| e.topics().contains(&Gateway::GmpCreated::SIGNATURE_HASH))
				.filter_map(|e| Gateway::GmpCreated::decode_log_data(e.data()).ok())
				.map(|e| e.id.into())
				.next()
				.context("Failed to send gmp message")?;
			tracing::info!("Sent tokens from {nw} to {nw2}, msg_id: {}", hex::encode(msg_id));
			msgs.push((*nw, msg_id));
		};
	}
	// Track messages
	let mut blocks = tc.finality_notification_stream();
	let mut id = None;
	loop {
		let (hash, _) = blocks.next().await.context("expected block")?;
		let mut traces: Vec<MessageTrace> = vec![];
		for (nw, msg_id) in &msgs {
			let trace = &tc
				.message_trace(*nw, *msg_id, hash)
				.await
				.context("failed to get message trace")?;
			traces.push(trace.clone());
		}
		let executed = traces.iter().filter_map(|t| t.exec.clone()).count();
		tracing::info!("waiting for messages to be executed");
		id = Some(tc.print_table(id, "message", traces).await?);
		if executed == msgs.len() - 1 {
			break;
		}
	}
	// Check resulting balances
	for (i, (_nw, token, callee, _gas_limit)) in contracts.iter().enumerate() {
		let alice_bal = token.balanceOf(ALICE).call().await?;
		let bob_bal = token.balanceOf(MINTER).call().await?;
		// On every chain, ALICE now has -=TRANSFER_AMOUNT
		assert_eq!(alice_bal, alice_balances[i] - U256::from(TRANSFER_AMOUNT));
		let received_amount = if i == 1 {
			// insufficient gas_limit: call fails, MINTER gets 0
			U256::ZERO
		} else {
			// sufficient gas_limit: call succeeds, MINTER gets TRANSFER_AMOUNT
			U256::from(TRANSFER_AMOUNT)
		};
		assert_eq!(callee.total().call().await?, received_amount);
		assert_eq!(bob_bal, received_amount);
	}

	Ok(())
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

	common::test_oats_sender(contracts, tc).await
}

#[tokio::test]
#[ignore]
async fn forever() -> Result<()> {
	let (_env, _tc) = TestEnv::new(Backend::Evm, false).await?;
	tracing::info!("Test env ready. Keeping live indefinitely...");
	#[allow(clippy::empty_loop)]
	loop {}
}
