use crate::tests::sr25519::Public;
use crate::{worker::TaskExecutor, TaskExecutorParams};
use futures::channel::mpsc;
use sc_keystore::LocalKeystore;
use sc_network_test::Block;
use sc_network_test::TestClientBuilderExt;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::sr25519;
use sp_core::Pair;
use std::marker::PhantomData;
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

async fn build_ethereum_worker() -> TaskExecutorType {
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
		connector_url: Some("http://rosetta.analog.one:8081".into()),
		connector_blockchain: Some("ethereum".into()),
		connector_network: Some("dev".into()),
	};

	TaskExecutor::new(params).await.unwrap()
}

async fn build_astar_worker() -> TaskExecutorType {
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
		sign_data_sender,
		connector_url: Some("http://127.0.0.1:8083".into()),
		connector_blockchain: Some("astar".into()),
		connector_network: Some("dev".into()),
	};

	TaskExecutor::new(params).await.unwrap()
}

#[tokio::test]
// Ethereum localnet contract call
async fn task_executor_ethereum_sc_call() {
	let worker = build_ethereum_worker().await;
	let address = "0x3de7086ce750513ef79d14eacbd1282c4e4b0cea";
	let function = "function get_votes_stats() external view returns (uint, uint)";
	let input: Vec<String> = vec![];
	let output_len = 2;
	let data = worker.call_eth_contract(address, function, &input).await.unwrap();
	let return_values = data.result.as_array().unwrap();
	assert!(return_values.len() == output_len);
}

#[tokio::test]
// Astar localnet contract call
async fn task_executor_astar_sc_call() {
	let worker = build_astar_worker().await;
	
	// replace this address with your own deployed contract address
	let address = "0x856478e9438f00ebf88e93ac2c76b3bd4c8e768a";
	let function = "function get_votes_stats() external view returns (uint, uint)";
	let input: Vec<String> = vec![];
	let output_len = 2;
	let data = worker.call_eth_contract(address, function, &input).await.unwrap();
	let return_values = data.result.as_array().unwrap();
	assert!(return_values.len() == output_len);
}
