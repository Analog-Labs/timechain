use anyhow::Result;
use e2e_tests::{Backend, TestEnv};
use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
use serde_json::Value;

#[tokio::test]
async fn test_eth_rpc_connection() -> Result<()> {
	// In the test environment, the Ethereum RPC is running on localhost:8545
	// This is set up in TestEnvBuilder
	let eth_rpc_url = "http://localhost:8545";
	println!("Connecting to eth-rpc at: {}", eth_rpc_url);

	// Create a JSON-RPC client
	let client = HttpClientBuilder::default().build(eth_rpc_url)?;

	// Test basic eth_chainId RPC call
	let chain_id: String = client.request("eth_chainId", rpc_params![]).await?;
	println!("Connected to Ethereum chain with ID: {}", chain_id);

	// Test eth_blockNumber RPC call
	let block_number: String = client.request("eth_blockNumber", rpc_params![]).await?;
	println!("Current block number: {}", block_number);

	// Test eth_accounts RPC call to get available accounts
	let accounts: Vec<String> = client.request("eth_accounts", rpc_params![]).await?;
	println!("Available accounts: {:?}", accounts);

	assert!(accounts.is_empty(), "Expect 0 Pre Funded Ethereum Accounts");

	// Test eth_getBlockByNumber to verify block production
	let block: Value = client.request("eth_getBlockByNumber", rpc_params!["latest", false]).await?;
	println!("Latest block: {}", serde_json::to_string_pretty(&block)?);

	// Verify the block has expected fields
	assert!(block.get("number").is_some(), "Block should have a number field");
	assert!(block.get("hash").is_some(), "Block should have a hash field");

	Ok(())
}
