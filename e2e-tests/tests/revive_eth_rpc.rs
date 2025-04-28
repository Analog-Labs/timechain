use anyhow::{Context, Result};
use jsonrpsee::http_client::HttpClientBuilder;
use pallet_revive::evm::{Account, BlockTag};
use pallet_revive_eth_rpc::EthRpcClient;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child as TokioChild, Command as TokioCommand};
use tokio::time::sleep;

/// Test that connects to a running pallet-revive-eth-rpc node and performs basic Ethereum JSON-RPC calls
async fn test_revive_eth_rpc_connection(rpc_url: &str) -> Result<()> {
	// Create a client using the EthRpcClient trait
	let client = Arc::new(HttpClientBuilder::default().build(rpc_url)?);

	// Create a default account for testing
	let account = Account::default();
	println!("Account address: {:?}", account.address());
	println!("Substrate account: {}", account.substrate_account());

	// Test chain ID
	let chain_id = client.chain_id().await?;
	println!("Chain ID: {chain_id:?}");

	// Test latest block
	let block = client.get_block_by_number(BlockTag::Latest.into(), false).await?;
	println!("Latest block: {block:#?}");

	// Test account nonce
	let nonce = client.get_transaction_count(account.address(), BlockTag::Latest.into()).await?;
	println!("Account nonce: {nonce:?}");

	// Test account balance
	let balance = client.get_balance(account.address(), BlockTag::Latest.into()).await?;
	println!("Account balance: {balance:?}");

	// Test sync state
	let sync_state = client.syncing().await?;
	println!("Sync state: {sync_state:?}");

	// Test accounts
	let accounts = client.accounts().await?;
	println!("Accounts: {accounts:?}");
	assert!(!accounts.is_empty(), "Expected at least one account");

	Ok(())
}

/// Starts the pallet-revive-eth-rpc node in a separate process
fn start_revive_eth_rpc_node() -> Result<Child> {
	println!("Starting pallet-revive-eth-rpc node");

	// Start the node process
	Command::new("cargo")
		.env("RUST_LOG", "info,eth-rpc=debug")
		.args(["run", "--release", "-p", "pallet-revive-eth-rpc", "--", "--dev"])
		.spawn()
		.context("Failed to start pallet-revive-eth-rpc node")
}

/// Starts the node and waits for it to be ready before testing
async fn start_and_wait_for_node(rpc_url: &str) -> Result<()> {
	// Start the node process
	let mut node_process = start_revive_eth_rpc_node()?;

	// Wait for the node to start up
	println!("Waiting for node to start up...");
	sleep(Duration::from_secs(10)).await;

	// Make sure to kill the process when the test ends
	let result = tokio::select! {
		_ = tokio::signal::ctrl_c() => {
			println!("Received Ctrl+C, shutting down...");
			Ok(())
		}
		result = test_revive_eth_rpc_connection(rpc_url) => result,
	};

	// Kill the node process
	let _ = node_process.kill();

	result
}

#[tokio::test]
async fn revive_eth_rpc() -> Result<()> {
	// The URL where the pallet-revive-eth-rpc will be running
	let eth_rpc_url = "http://127.0.0.1:8545";

	start_and_wait_for_node(eth_rpc_url).await
}
