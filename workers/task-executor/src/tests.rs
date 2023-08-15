use crate::{TaskExecutor, TaskExecutorParams};
use anyhow::Result;
use futures::channel::mpsc;
use sc_network_test::{Block, TestClientBuilder, TestClientBuilderExt};
use sp_api::{ApiRef, ProvideRuntimeApi};
use std::marker::PhantomData;
use std::sync::Arc;
use time_primitives::{PeerId, ShardId, TaskDescriptor, TaskExecution, TaskId, TimeApi};

#[derive(Clone, Default)]
struct MockApi;

impl MockApi {}

sp_api::mock_impl_runtime_apis! {
	impl TimeApi<Block> for MockApi {
		fn get_shards(&self, _peer_id: PeerId) -> Vec<ShardId> {
			vec![1]
		}

		fn get_shard_tasks(&self, _shard_id: ShardId) -> Vec<TaskExecution> {
			vec![]
		}

		fn get_task(&self, _task_id: TaskId) -> Option<TaskDescriptor> {
			None
		}
	}
}

impl ProvideRuntimeApi<Block> for MockApi {
	type Api = Self;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.clone().into()
	}
}

#[tokio::test]
async fn task_executor_smoke() -> Result<()> {
	let (sign_data_sender, _sign_data_receiver) = mpsc::channel(400);

	let (client, backend) = {
		let builder = TestClientBuilder::with_default_backend();
		let backend = builder.backend();
		let (client, _) = builder.build_with_longest_chain();
		(Arc::new(client), backend)
	};

	let api = Arc::new(MockApi::default());

	let task_spawner = futures::executor::block_on(crate::Task::new(crate::TaskSpawnerParams {
		tss: sign_data_sender,
		connector_url: Some("".into()),
		connector_blockchain: Some("".into()),
		connector_network: Some("".into()),
	}))
	.unwrap();

	let params = TaskExecutorParams {
		_block: PhantomData::default(),
		backend,
		client,
		runtime: api,
		peer_id: [0u8; 32],
		task_spawner,
	};

	let _task_executor = TaskExecutor::new(params);
	Ok(())
}
