use alloy::primitives::address;
use alloy::providers::WsConnect;
use alloy::sol;
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
use time_primitives::{Address32, MessageId};

// Anvil's default accounts
const ALICE: Address20 = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
const ALICE_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const BOB: Address20 = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");

// Codegen from ABI file to interact with the contract.
sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[sol(rpc)]
	OmniToken,
	"contracts/OmniToken.json"
);

type Address20 = alloy::primitives::Address;

fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}

#[tokio::test]
async fn oats_evm() -> Result<()> {
	const TRANSFER_AMOUNT: u64 = 10u64.pow(18);
	const CAP_AMOUNT: u64 = 5 * TRANSFER_AMOUNT;

	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	let block = tc.latest_block().await?.0;

	let mut contracts = vec![];
	// Deploy Token to every network
	for nw in tc.networks(block).await? {
		let gw = nw.info.unwrap().gateway;
		let nw_id = nw.network;
		let c = env.chain_container(nw_id).unwrap();

		let port = c.get_host_port_ipv4(8545).await.unwrap();
		let ws = WsConnect::new(format!("ws://localhost:{port}"));
		let signer: PrivateKeySigner = ALICE_KEY.parse()?;
		let wallet = EthereumWallet::from(signer.clone());
		let rpc = Arc::new(ProviderBuilder::new().wallet(wallet).connect_ws(ws).await?);

		let token = OmniToken::deploy(
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
	// Set OMNI token networks
	for (nw, token) in contracts.iter() {
		for (n, t) in contracts.iter().filter(|(n, _)| n.ne(nw)) {
			token.set_network(*n, *t.address()).send().await?.get_receipt().await?;
		}
	}
	// Check initial balances
	let mut alice_balances = vec![];
	for (_nw, token) in contracts.iter() {
		let alice_bal = token.balanceOf(ALICE).call().await?;
		let bob_bal = token.balanceOf(BOB).call().await?;
		// On every chain, ALICE has some OMNI tokens, and BOB has none.
		assert_ne!(alice_bal, U256::ZERO);
		assert_eq!(bob_bal, U256::ZERO);
		alice_balances.push(alice_bal);
	}
	// Transfer tokens from every network to next network, ring way
	let mut msgs = vec![];
	let mut ring = contracts.iter().cycle().take(contracts.len() + 1).peekable();
	while let Some((nw, token)) = ring.next() {
		if let Some((nw2, _)) = ring.peek() {
			let gmp_fee = token.cost(*nw2).call().await?;
			let receipt = token
				.send(*nw2, BOB, U256::from(TRANSFER_AMOUNT))
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
		if executed == msgs.len() {
			break;
		}
	}
	// Check resulting balances
	for (i, (_nw, token)) in contracts.iter().enumerate() {
		let alice_bal = token.balanceOf(ALICE).call().await?;
		let bob_bal = token.balanceOf(BOB).call().await?;
		// On every chain, ALICE now has -=TRANSFER_AMOUNT, BOB has TRANSFER_AMOUNT
		assert_eq!(alice_bal, alice_balances[i] - U256::from(TRANSFER_AMOUNT));
		assert_eq!(bob_bal, U256::from(TRANSFER_AMOUNT));
	}

	Ok(())
}

#[tokio::test]
#[ignore]
async fn forever() -> Result<()> {
	let (_env, _tc) = TestEnv::new(Backend::Evm, false).await?;
	tracing::info!("Test env ready. Keeping live indefinitely...");
	#[allow(clippy::empty_loop)]
	loop {}
}
