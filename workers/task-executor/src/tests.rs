use crate::tests::sr25519::Public;
use crate::{worker::TaskExecutor, TaskExecutorParams};
use anyhow::Result;
use ethers_solc::artifacts::Source;
use ethers_solc::{CompilerInput, EvmVersion, Solc};
use futures::channel::mpsc;
use rosetta_client::{create_wallet, EthereumExt};
use rosetta_docker::Env;
use sc_keystore::LocalKeystore;
use sc_network_test::Block;
use sc_network_test::TestClientBuilderExt;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::sr25519;
use sp_core::Pair;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;
use substrate_test_runtime_client::sc_client_db::Backend;
use substrate_test_runtime_client::{
	runtime::{AccountId, BlockNumber},
	TestClientBuilder,
};
use time_primitives::sharding::Shard;
use time_primitives::TimeApi;

type TaskExecutorType = TaskExecutor<Block, Backend<Block>, TestApi, Public, BlockNumber>;

/// Set of test accounts using [`time_primitives::crypto`] types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
enum TimeKeyring {
	Alice,
	Bob,
	Charlie,
}

impl TimeKeyring {
	/// Return key pair.
	pub fn pair(self) -> sr25519::Pair {
		sr25519::Pair::from_string(self.to_seed().as_str(), None).unwrap()
	}

	/// Return public key.
	pub fn public(self) -> sr25519::Public {
		self.pair().public()
	}

	/// Return seed string.
	pub fn to_seed(self) -> String {
		format!("//{self}")
	}
}

#[derive(Clone)]
pub(crate) struct RuntimeApi {}

sp_api::mock_impl_runtime_apis! {
	impl TimeApi<Block, AccountId, BlockNumber> for RuntimeApi {
		fn get_shards(&self) -> Vec<(u64, Shard)> {
			vec![(1, Shard::Three([
				TimeKeyring::Alice.public().into(),
				TimeKeyring::Bob.public().into(),
				TimeKeyring::Charlie.public().into(),
			]))]
		}
	}
}

#[derive(Default, Clone)]
pub(crate) struct TestApi {}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = RuntimeApi;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		RuntimeApi {}.into()
	}
}

async fn build_worker(url: &str, blockchain: &str, network: &str) -> TaskExecutorType {
	let (sign_data_sender, _sign_data_receiver) = mpsc::channel(400);
	let runtime_api = TestApi::default();
	let keystore = Arc::new(LocalKeystore::in_memory());

	// Create an observer.
	let (_client, backend) = {
		let builder = TestClientBuilder::with_default_backend();
		let backend = builder.backend();
		let (client, _) = builder.build_with_longest_chain();
		(Arc::new(client), backend)
	};

	let params = TaskExecutorParams {
		backend,
		runtime: runtime_api.into(),
		kv: keystore,
		_block: PhantomData::default(),
		account_id: PhantomData::default(),
		_block_number: PhantomData::default(),
		sign_data_sender,
		connector_url: Some(url.into()),
		connector_blockchain: Some(blockchain.into()),
		connector_network: Some(network.into()),
	};

	TaskExecutor::new(params).await.unwrap()
}

#[tokio::test]
// Ethereum localnet contract call
async fn task_executor_ethereum_sc_call() {
	let blockchain = "ethereum";
	let network = "dev";

	let config = rosetta_client::create_config(blockchain, network).unwrap();
	let env = Env::new("ethereum-sc-call", config).await.unwrap();
	let url = env.connector_url();

	let worker = build_worker(&url, blockchain, network).await;
	let contract_address = deploy_eth_testnet_contract(&url, blockchain, network).await.unwrap();

	let function = "function identity(bool a) returns (bool)";
	let input = ["true".into()];
	let data = worker.call_eth_contract(&contract_address, function, &input).await.unwrap();
	println!("data: {:?}", data);
	let result: Vec<String> = serde_json::from_value(data.result).unwrap();
	assert_eq!(result[0], "true");
}

#[tokio::test]
// Astar localnet contract call
async fn task_executor_astar_sc_call() {
	let blockchain = "astar";
	let network = "dev";

	let config = rosetta_client::create_config(blockchain, network).unwrap();
	let env = Env::new("astar-sc-call", config).await.unwrap();
	let url = env.connector_url();

	let worker = build_worker(&url, blockchain, network).await;
	let contract_address = deploy_eth_testnet_contract(&url, blockchain, network).await.unwrap();

	let function = "function identity(bool a) returns (bool)";
	let input = ["true".into()];
	let data = worker.call_eth_contract(&contract_address, function, &input).await.unwrap();
	let result: Vec<String> = serde_json::from_value(data.result).unwrap();
	assert_eq!(result[0], "true");
}

async fn deploy_eth_testnet_contract(url: &str, blockchain: &str, network: &str) -> Result<String> {
	let wallet =
		create_wallet(Some(blockchain.into()), Some(network.into()), Some(url.into()), None)
			.await?;
	wallet.faucet(1000000000000000).await?;
	let bytes = compile_snippet(
		r#"
            function identity(bool a) public view returns (bool) {
                return a;
            }
        "#,
	)?;

	let response = wallet.eth_deploy_contract(bytes).await?;
	let receipt = wallet.eth_transaction_receipt(&response.hash).await?;
	let contract_address = receipt.result["contractAddress"].as_str().unwrap();

	println!("contract_address: {:?}", contract_address);

	Ok(contract_address.into())
}

fn compile_snippet(source: &str) -> Result<Vec<u8>> {
	let solc = Solc::default();
	let source = format!("contract Contract {{ {source} }}");
	let mut sources = BTreeMap::new();
	sources.insert(Path::new("contract.sol").into(), Source::new(source));
	let input = CompilerInput::with_sources(sources)[0]
		.clone()
		.evm_version(EvmVersion::Homestead);
	let output = solc.compile_exact(&input)?;
	let file = output.contracts.get("contract.sol").unwrap();
	let contract = file.get("Contract").unwrap();
	let bytecode = contract
		.evm
		.as_ref()
		.unwrap()
		.bytecode
		.as_ref()
		.unwrap()
		.object
		.as_bytes()
		.unwrap()
		.to_vec();
	Ok(bytecode)
}
#[cfg(test)]
mod tests {
	use timechain_integration::query::{collect_data, CollectData};

	use graphql_client::{GraphQLQuery, Response as GraphQLResponse};

	#[tokio::test]
	async fn test_collect_data() {
		// Prepare the input variables
		let variables = collect_data::Variables {
			collection: "QmWVZN1S6Yhygt35gQej6e3VbEEffbrVuqZZCQc772uRt7".to_owned(),
			block: 1,
			cycle: 17,
			task_id: 3,
			data: vec!["1".to_owned()],
		};

		// Build the GraphQL request
		let request = CollectData::build_query(variables);

		// Execute the GraphQL request
		let response = reqwest::Client::new()
			.post("http://localhost:8009/graphql")
			.json(&request)
			.send()
			.await
			.expect("Failed to send request")
			.json::<GraphQLResponse<collect_data::ResponseData>>()
			.await
			.expect("Failed to parse response");

		match &response.data {
			Some(data) => {
				println!("{:?}", data.collect.status);

				println!("{:?}", data.collect);
			},
			None => println!("no deta found"),
		};
	}
}
