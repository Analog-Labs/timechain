use rosetta_client::{create_wallet, EthereumExt};

#[tokio::main]
async fn main() {
	let wallet = create_wallet(
		Some("astar".to_owned()),
		Some("dev".to_owned()),
		Some("http://rosetta.analog.one:8083".to_owned()),
		None,
	)
	.await
	.unwrap();

    wallet.faucet(1000000000000000).await.unwrap();
	let compiled_contract_bin =
		include_str!("voting_contract.bin").strip_suffix('\n').unwrap();
	let bytes = hex::decode(compiled_contract_bin).unwrap();

	//deploying contract
	let response = wallet.eth_deploy_contract(bytes).await.unwrap();

	//getting contract address
	let tx_receipt = wallet.eth_transaction_receipt(&response.hash).await.unwrap();
	let contract_address = tx_receipt.result["contractAddress"].clone();
	println!("Deployed contract address: {}", contract_address);
}
