use crate::{worker::TaskExecutor, TaskExecutorParams};
use futures::channel::mpsc;
// use sc_client_api::in_mem::Backend;
use substrate_test_runtime_client::sc_client_db::Backend;
use sc_keystore::LocalKeystore;
use sc_network_test::Block;
use sc_network_test::TestClientBuilderExt;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::sr25519;
use sp_core::Pair;
use std::marker::PhantomData;
use std::sync::Arc;
use substrate_test_runtime_client::{runtime::AccountId, TestClientBuilder};
use time_db::DatabaseConnection;
use time_primitives::sharding::Shard;
use time_primitives::TimeApi;
use crate::tests::sr25519::Public;

// pub struct Public(pub [u8; 32]);
type TaskExecutorType = TaskExecutor<Block, Backend<Block>, TestApi, Public>;

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
	impl TimeApi<Block, AccountId> for RuntimeApi {
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

async fn build_worker() -> TaskExecutorType {
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
		accountid: PhantomData::default(),
		sign_data_sender,
		connector_url: Some("http://rosetta.analog.one:8081".into()),
		connector_blockchain: Some("ethereum".into()),
		connector_network: Some("dev".into()),
		db: Some(DatabaseConnection::default()),
	};

	let worker = TaskExecutor::new(params).await.unwrap();
    worker 
}

#[tokio::test]
// Ethereum localnet contract call
async fn task_executor_ethereum_sc_call() {
    let worker = build_worker().await;
	let address = "0x678ea0447843f69805146c521afcbcc07d6e28a2";
	let function = "function get_votes_stats() external view returns (uint, uint)";
	let input: Vec<String> = vec![];
	let output_len = 2;
	let data = worker.call_eth_contract(address, function, &input).await.unwrap();
	let return_values = data.result.as_array().unwrap();
	assert!(return_values.len() == output_len);
}

#[tokio::test]
#[ignore]
//astar connector is under development will update below function if needed when its done.
async fn task_executor_astar_sc_call() {
    let worker = build_worker().await;
	let address = "0x678ea0447843f69805146c521afcbcc07d6e28a2";
	let function = "function get_votes_stats() external view returns (uint, uint)";
	let input: Vec<String> = vec![];
	let output_len = 2;
	let data = worker.call_eth_contract(address, function, &input).await.unwrap();
	let return_values = data.result.as_array().unwrap();
	assert!(return_values.len() == output_len);
}
