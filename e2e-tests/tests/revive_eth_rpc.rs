use anyhow::{Context, Result};
use e2e_tests::{Backend, TestEnv, Tester};
use jsonrpsee::http_client::HttpClientBuilder;
use pallet_revive::evm::{Account, BlockTag};
use pallet_revive_eth_rpc::EthRpcClient;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test that connects to a running timechain node and performs basic Ethereum JSON-RPC calls
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

#[tokio::test]
async fn revive_eth_rpc() -> Result<()> {
	// Create a test environment with the timechain node
	let (env, _tester) = TestEnv::new(Backend::Evm, false)?;

	// Get the validator container and its URL
	let validator = env.validator_container();
	let host = validator.get_host().await?;
	// TODO: run in chronicle by default or make it conditional using feature = testing
	let port = validator.get_mapped_port(8545).await?;

	// The URL where the Ethereum JSON-RPC is exposed on the timechain node
	let eth_rpc_url = format!("http://{}:{}", host, port);
	println!("Connecting to Ethereum JSON-RPC at: {}", eth_rpc_url);

	// Wait for the node to fully start up
	println!("Waiting for node to start up...");
	sleep(Duration::from_secs(10)).await;

	// Run the Ethereum RPC tests
	test_revive_eth_rpc_connection(&eth_rpc_url).await
}
