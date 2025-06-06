use alloy::primitives::{address, Bytes};
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

const TRANSFER_AMOUNT: u64 = 10u64.pow(18);
const CAP_AMOUNT: u64 = 5 * TRANSFER_AMOUNT;

// Codegen from ABI file to interact with the contract.
sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[sol(rpc)]
	OATSSender,
	"contracts/OATSSender.json"
);

sol!(
	#[allow(clippy::too_many_arguments)]
	#[allow(missing_docs)]
	#[sol(rpc)]
	OATSSenderCaller,
	"contracts/OATSSenderCaller.json"
);

sol! {
	#[allow(missing_docs)]
	#[sol(rpc, bytecode="0x60a060405234801561000f575f5ffd5b506040516105b43803806105b4833981810160405281019061003191906100c9565b8073ffffffffffffffffffffffffffffffffffffffff1660808173ffffffffffffffffffffffffffffffffffffffff1681525050506100f4565b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6100988261006f565b9050919050565b6100a88161008e565b81146100b2575f5ffd5b50565b5f815190506100c38161009f565b92915050565b5f602082840312156100de576100dd61006b565b5b5f6100eb848285016100b5565b91505092915050565b6080516104a961010b5f395f607901526104a95ff3fe608060405234801561000f575f5ffd5b5060043610610034575f3560e01c80632ddbd13a1461003857806388a7ca5c14610056575b5f5ffd5b610040610072565b60405161004d91906101a9565b60405180910390f35b610070600480360381019061006b91906102af565b610077565b005b5f5481565b7f000000000000000000000000000000000000000000000000000000000000000073ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610105576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016100fc9061038d565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168573ffffffffffffffffffffffffffffffffffffffff1603610173576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161016a906103f5565b60405180910390fd5b825f5f8282546101839190610440565b925050819055505050505050565b5f819050919050565b6101a381610191565b82525050565b5f6020820190506101bc5f83018461019a565b92915050565b5f5ffd5b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6101f3826101ca565b9050919050565b610203816101e9565b811461020d575f5ffd5b50565b5f8135905061021e816101fa565b92915050565b61022d81610191565b8114610237575f5ffd5b50565b5f8135905061024881610224565b92915050565b5f5ffd5b5f5ffd5b5f5ffd5b5f5f83601f84011261026f5761026e61024e565b5b8235905067ffffffffffffffff81111561028c5761028b610252565b5b6020830191508360018202830111156102a8576102a7610256565b5b9250929050565b5f5f5f5f5f608086880312156102c8576102c76101c2565b5b5f6102d588828901610210565b95505060206102e688828901610210565b94505060406102f78882890161023a565b935050606086013567ffffffffffffffff811115610318576103176101c6565b5b6103248882890161025a565b92509250509295509295909350565b5f82825260208201905092915050565b7f556e617574686f72697a656400000000000000000000000000000000000000005f82015250565b5f610377600c83610333565b915061038282610343565b602082019050919050565b5f6020820190508181035f8301526103a48161036b565b9050919050565b7f4661696c656400000000000000000000000000000000000000000000000000005f82015250565b5f6103df600683610333565b91506103ea826103ab565b602082019050919050565b5f6020820190508181035f83015261040c816103d3565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f61044a82610191565b915061045583610191565b925082820190508082111561046d5761046c610413565b5b9291505056fea2646970667358221220721001adf061100607ede876419d92d82c138b64609feee52e4f89fa20a57e2764736f6c634300081c0033")]
	contract Callee {
		 uint256 public total;
		 address immutable _token;

		 constructor(address token) {
			 _token = token;
		 }

		 function onTransferReceived(address from, address, uint256 amount, bytes calldata) external {
			 require(msg.sender == _token, "Unauthorized");
			 require(from != address(0), "Failed");

			 total += amount;
		 }
	 }
}

type Address20 = alloy::primitives::Address;

fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}

#[tokio::test]
#[ignore]
async fn oats_sender_caller_evm() -> Result<()> {
	let (env, tc) = TestEnv::new(Backend::Evm, false).await?;
	let block = tc.latest_block().await?.0;

	const GAS_LIMIT_STEP: u64 = 50_000;

	let mut contracts = vec![];
	// Deploy Token + Callee to every network
	for (i, nw) in tc.networks(block).await?.into_iter().enumerate() {
		let gw = nw.info.unwrap().gateway;
		let nw_id = nw.network;
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

		contracts.push((nw_id, token, callee, U256::from(GAS_LIMIT_STEP * (i as u64 + 1))));
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
		let bob_bal = token.balanceOf(BOB).call().await?;
		// On every chain, ALICE has some OMNI tokens, and BOB has none.
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
					BOB,
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
		let bob_bal = token.balanceOf(BOB).call().await?;
		// On every chain, ALICE now has -=TRANSFER_AMOUNT
		assert_eq!(alice_bal, alice_balances[i] - U256::from(TRANSFER_AMOUNT));
		let received_amount = if i == 1 {
			// insufficient gas_limit: call fails, BOB gets 0
			U256::ZERO
		} else {
			// sufficient gas_limit: call succeeds, BOB gets TRANSFER_AMOUNT
			U256::from(TRANSFER_AMOUNT)
		};
		assert_eq!(callee.total().call().await?, received_amount);
		assert_eq!(bob_bal, received_amount);
	}

	Ok(())
}

#[tokio::test]
#[ignore]
async fn oats_sender_evm() -> Result<()> {
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
		// On every chain, ALICE now has -=U256::from(TRANSFER_AMOUNT), BOB has U256::from(TRANSFER_AMOUNT)
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
