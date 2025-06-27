#![allow(dead_code)]

use std::sync::Arc;

use alloy::network::EthereumWallet;
use alloy::primitives::address;
use alloy::primitives::Bytes;
use alloy::primitives::U256;
use alloy::providers::fillers::BlobGasFiller;
use alloy::providers::fillers::ChainIdFiller;
use alloy::providers::fillers::FillProvider;
use alloy::providers::fillers::GasFiller;
use alloy::providers::fillers::JoinFill;
use alloy::providers::fillers::NonceFiller;
use alloy::providers::fillers::WalletFiller;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::providers::RootProvider;
use alloy::providers::WsConnect;
use alloy::signers::local::PrivateKeySigner;
use alloy::sol;
use alloy::sol_types::SolEvent;
use anyhow::{Context, Result};
use e2e_tests::Tester;
use futures::stream::StreamExt;
use gmp::Gateway;
use Callee::CalleeInstance;
use OATSSender::OATSSenderInstance;

use tc_cli::{MessageTrace, NetworkId};
use time_primitives::{Address32, MessageId};
use OATSSenderCaller::OATSSenderCallerInstance;

// Anvil's default accounts
pub const ALICE: Address20 = address!("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
pub const ALICE_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
pub const MINTER: Address20 = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
pub const MINTER_KEY: &str = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

pub const TRANSFER_AMOUNT: u64 = 10u64.pow(18);
pub const CAP_AMOUNT: u64 = 10 * TRANSFER_AMOUNT;
pub const GAS_LIMIT_STEP: u64 = 50_000;

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
	#[sol(rpc, bytecode="0x60a060405234801561000f575f5ffd5b50604051610705380380610705833981810160405281019061003191906100c9565b8073ffffffffffffffffffffffffffffffffffffffff1660808173ffffffffffffffffffffffffffffffffffffffff1681525050506100f4565b5f5ffd5b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6100988261006f565b9050919050565b6100a88161008e565b81146100b2575f5ffd5b50565b5f815190506100c38161009f565b92915050565b5f602082840312156100de576100dd61006b565b5b5f6100eb848285016100b5565b91505092915050565b6080516105fa61010b5f395f60ef01526105fa5ff3fe608060405234801561000f575f5ffd5b506004361061004a575f3560e01c80632ddbd13a1461004e578063462943071461006c578063500470e414610088578063951fdd42146100b8575b5f5ffd5b6100566100e8565b6040516100639190610286565b60405180910390f35b610086600480360381019061008191906103c3565b6100ed565b005b6100a2600480360381019061009d9190610459565b610237565b6040516100af9190610286565b60405180910390f35b6100d260048036038101906100cd9190610459565b61024c565b6040516100df9190610286565b60405180910390f35b5f5481565b7f000000000000000000000000000000000000000000000000000000000000000073ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461017b576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610172906104de565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168573ffffffffffffffffffffffffffffffffffffffff16036101e9576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016101e090610546565b60405180910390fd5b8260015f8861ffff1661ffff1681526020019081526020015f205f8282546102119190610591565b92505081905550825f5f8282546102289190610591565b92505081905550505050505050565b6001602052805f5260405f205f915090505481565b5f60015f8361ffff1661ffff1681526020019081526020015f20549050919050565b5f819050919050565b6102808161026e565b82525050565b5f6020820190506102995f830184610277565b92915050565b5f5ffd5b5f5ffd5b5f61ffff82169050919050565b6102bd816102a7565b81146102c7575f5ffd5b50565b5f813590506102d8816102b4565b92915050565b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f610307826102de565b9050919050565b610317816102fd565b8114610321575f5ffd5b50565b5f813590506103328161030e565b92915050565b6103418161026e565b811461034b575f5ffd5b50565b5f8135905061035c81610338565b92915050565b5f5ffd5b5f5ffd5b5f5ffd5b5f5f83601f84011261038357610382610362565b5b8235905067ffffffffffffffff8111156103a05761039f610366565b5b6020830191508360018202830111156103bc576103bb61036a565b5b9250929050565b5f5f5f5f5f5f60a087890312156103dd576103dc61029f565b5b5f6103ea89828a016102ca565b96505060206103fb89828a01610324565b955050604061040c89828a01610324565b945050606061041d89828a0161034e565b935050608087013567ffffffffffffffff81111561043e5761043d6102a3565b5b61044a89828a0161036e565b92509250509295509295509295565b5f6020828403121561046e5761046d61029f565b5b5f61047b848285016102ca565b91505092915050565b5f82825260208201905092915050565b7f556e617574686f72697a656400000000000000000000000000000000000000005f82015250565b5f6104c8600c83610484565b91506104d382610494565b602082019050919050565b5f6020820190508181035f8301526104f5816104bc565b9050919050565b7f4661696c656400000000000000000000000000000000000000000000000000005f82015250565b5f610530600683610484565b915061053b826104fc565b602082019050919050565b5f6020820190508181035f83015261055d81610524565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f61059b8261026e565b91506105a68361026e565b92508282019050808211156105be576105bd610564565b5b9291505056fea26469706673582212203c1dff1e61ff2a72694324468f756fe36f18a9189e5800c496708a47f32a731064736f6c634300081c0033")]
	contract Callee {
		  address immutable _token;

		  uint256 public total;
		  mapping(uint16 => uint256) public totalByNetwork;

		  constructor(address token) {
			  _token = token;
		  }

		  function onTransferReceived(uint16 newtork, address from, address, uint256 amount, bytes calldata) external {
			  require(msg.sender == _token, "Unauthorized");
			  require(from != address(0), "Failed");

			  totalByNetwork[newtork] += amount;
			  total += amount;
		  }

		  function totalFrom(uint16 _newtork) public view returns (uint256) {
			  return totalByNetwork[_newtork];
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

type Rpc = FillProvider<
	JoinFill<
		JoinFill<
			alloy::providers::Identity,
			JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
		>,
		WalletFiller<EthereumWallet>,
	>,
	RootProvider,
>;

pub fn a_addr(address: Address32) -> Address20 {
	Address20::from_word(address.into())
}

pub async fn build_rpc(signer_key: &str, port: u16) -> Result<Arc<Rpc>> {
	let ws = WsConnect::new(format!("ws://localhost:{port}"));
	let signer: PrivateKeySigner = signer_key.parse()?;
	let wallet = EthereumWallet::from(signer.clone());
	ProviderBuilder::new()
		.wallet(wallet)
		.connect_ws(ws)
		.await
		.map(Arc::new)
		.map_err(Into::into)
}

pub async fn test_oats_sender<P: Provider>(
	contracts: Vec<(NetworkId, OATSSenderInstance<P>)>,
	tc: &Tester,
) -> Result<()> {
	// Set OMNI token networks
	for (nw, token) in contracts.iter() {
		for (n, t) in contracts.iter().filter(|(n, _)| n.ne(nw)) {
			token.set_network(*n, *t.address()).send().await?.get_receipt().await?;
		}
	}
	// Check initial balances
	let mut balances = vec![];
	for (_nw, token) in contracts.iter() {
		let minter_bal = token.balanceOf(MINTER).call().await?;
		let alice_bal = token.balanceOf(ALICE).call().await?;
		// On every chain, MINTER has some OMNI tokens
		assert_ne!(minter_bal, U256::ZERO);

		balances.push((alice_bal, minter_bal));
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
		// On every chain, MINTER now has -=U256::from(TRANSFER_AMOUNT), ALICE has +=U256::from(TRANSFER_AMOUNT)
		assert_eq!(minter_bal, balances[i].1 - U256::from(TRANSFER_AMOUNT));
		assert_eq!(alice_bal, balances[i].0 + U256::from(TRANSFER_AMOUNT));
	}

	Ok(())
}

pub async fn test_oats_sender_caller<P: Provider>(
	contracts: Vec<(NetworkId, OATSSenderCallerInstance<P>, CalleeInstance<P>, u64)>,
	tc: &Tester,
) -> Result<()> {
	// Set OMNI token networks
	for (nw, token, _, _) in contracts.iter() {
		for (n, t, _, _) in contracts.iter().filter(|(n, _, _, _)| n.ne(nw)) {
			token.set_network(*n, *t.address()).send().await?.get_receipt().await?;
		}
	}
	// Check initial balances
	let mut balances = vec![];
	for (_nw, token, callee, _) in contracts.iter() {
		let alice_bal = token.balanceOf(ALICE).call().await?;
		let minter_bal = token.balanceOf(MINTER).call().await?;
		balances.push((alice_bal, minter_bal));
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
					ALICE,
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
		let minter_bal = token.balanceOf(MINTER).call().await?;
		// On every chain, MINTER now has -=TRANSFER_AMOUNT
		assert_eq!(minter_bal, balances[i].1 - U256::from(TRANSFER_AMOUNT));
		let received_amount = if i == 1 {
			// insufficient gas_limit: call fails, ALICE gets 0
			U256::ZERO
		} else {
			// sufficient gas_limit: call succeeds, ALICE gets TRANSFER_AMOUNT
			U256::from(TRANSFER_AMOUNT)
		};
		assert_eq!(alice_bal, balances[i].0 + received_amount);
		assert_eq!(callee.total().call().await?, received_amount);
	}

	Ok(())
}
