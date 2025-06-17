#![allow(dead_code)]

use alloy::primitives::address;
use alloy::providers::Provider;
use alloy::sol;
use alloy::sol_types::SolEvent;
use alloy::primitives::U256;
use anyhow::{Context, Result};
use e2e_tests::Tester;
use futures::stream::StreamExt;
use gmp::Gateway;
use OATSSender::OATSSenderInstance;

use tc_cli::{MessageTrace, NetworkId};
use time_primitives::{Address32, MessageId};

// Anvil's default accounts
pub const ALICE: Address20 = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
pub const ALICE_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
pub const MINTER: Address20 = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
pub const MINTER_KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

pub const TRANSFER_AMOUNT: u64 = 10u64.pow(18);
pub const CAP_AMOUNT: u64 = 10 * TRANSFER_AMOUNT;

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

sol! {
	#[allow(missing_docs)]
	#[sol(rpc)]
	interface IERC20 {
		function mint(address to, uint256 amount) public;
		function balanceOf(address account) external view returns (uint256);
		function totalSupply() external view returns (uint256);
		function decimals() public pure override returns (uint8);
		function symbol() public view virtual returns (string memory);
		function cap() public view virtual returns (uint256);
	}
}

pub type Address20 = alloy::primitives::Address;

pub fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}


pub async fn test_oats_sender<P: Provider>(
	contracts: Vec<(NetworkId, OATSSenderInstance<P>)>,
	tc: Tester,
) -> Result<()> {
	// Set OMNI token networks
	for (nw, token) in contracts.iter() {
		for (n, t) in contracts.iter().filter(|(n, _)| n.ne(nw)) {
			token.set_network(*n, *t.address()).send().await?.get_receipt().await?;
		}
	}
	// Check initial balances
	let mut minter_balances = vec![];
	for (_nw, token) in contracts.iter() {
		let minter_bal = token.balanceOf(MINTER).call().await?;
		let alice_bal = token.balanceOf(ALICE).call().await?;
		// On every chain, MINTER has some OMNI tokens, and ALICE has none.
		assert_ne!(minter_bal, U256::ZERO);
		assert_eq!(alice_bal, U256::ZERO);
		minter_balances.push(minter_bal);
	}
	// Transfer tokens from every network to next network, ring way
	let mut msgs = vec![];
	let mut ring = contracts.iter().cycle().take(contracts.len() + 1).peekable();
	while let Some((nw, token)) = ring.next() {
		if let Some((nw2, _)) = ring.peek() {
			let gmp_fee = token.cost(*nw2).call().await?;
			let receipt = token
				.send(*nw2, ALICE, U256::from(TRANSFER_AMOUNT))
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
		let minter_bal = token.balanceOf(MINTER).call().await?;
		let alice_bal = token.balanceOf(ALICE).call().await?;
		// On every chain, MINTER now has -=U256::from(TRANSFER_AMOUNT), ALICE has U256::from(TRANSFER_AMOUNT)
		assert_eq!(minter_bal, minter_balances[i] - U256::from(TRANSFER_AMOUNT));
		assert_eq!(alice_bal, U256::from(TRANSFER_AMOUNT));
	}

	Ok(())
}
