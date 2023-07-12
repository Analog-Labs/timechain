use rosetta_client::{create_wallet, EthereumExt};

#[tokio::main]
async fn main() {
	let eth_contract_address = deploy_contract("ethereum", "dev", "http://127.0.0.1:8080").await;
	println!("Contract deployed for eth at: {}", eth_contract_address);
	let astr_contrac_address = deploy_contract("astar", "dev", "http://127.0.0.1:8081").await;
	println!("Contract deployed for astar at: {}", astr_contrac_address);
	let eth_test_contract_address =
		deploy_test_contract("ethereum", "dev", "http://127.0.0.1:8080").await;
	println!("Contract test deployed for eth at: {}", eth_test_contract_address);
}

async fn deploy_contract(blockchain: &str, network: &str, url: &str) -> String {
	let wallet = create_wallet(
		Some(blockchain.to_owned()),
		Some(network.to_owned()),
		Some(url.to_owned()),
		None,
	)
	.await
	.unwrap();

	wallet.faucet(1000000000000000).await.unwrap();
	let compiled_contract_bin = include_str!("voting_contract.bin").strip_suffix('\n').unwrap();
	let bytes = hex::decode(compiled_contract_bin).unwrap();

	//deploying contract
	let response = wallet.eth_deploy_contract(bytes).await.unwrap();

	//getting contract address
	let tx_receipt = wallet.eth_transaction_receipt(&response.hash).await.unwrap();
	let contract_address = tx_receipt.result["contractAddress"].clone();
	contract_address.to_string()
}

async fn deploy_test_contract(blockchain: &str, network: &str, url: &str) -> String {
	let wallet = create_wallet(
		Some(blockchain.to_owned()),
		Some(network.to_owned()),
		Some(url.to_owned()),
		None,
	)
	.await
	.unwrap();

	wallet.faucet(1000000000000000).await.unwrap();
	let compiled_contract_bin = include_str!("test_contract.bin").strip_suffix('\n');
	let bytes = hex::decode(compiled_contract_bin.unwrap()).unwrap();

	//deploying contract
	let response = wallet.eth_deploy_contract(bytes).await.unwrap();

	//getting contract address
	let tx_receipt = wallet.eth_transaction_receipt(&response.hash).await.unwrap();
	let contract_address = tx_receipt.result["contractAddress"].clone();
	contract_address.to_string()
}
