use crate::{TaskExecutor, TaskExecutorParams};
use anyhow::Result;
use futures::{future, FutureExt};
use sc_network_test::{Block, TestClientBuilder, TestClientBuilderExt};
use sp_api::{ApiRef, ProvideRuntimeApi};
use std::marker::PhantomData;
use std::sync::Arc;
use std::{future::Future, pin::Pin};
use time_primitives::{PeerId, ShardId, TaskCycle, TaskDescriptor, TaskExecution, TaskId, TaskSpawner, TimeApi, TssSignature};

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

struct MockTask {}

#[async_trait::async_trait]
impl TaskSpawner for MockTask {
	async fn block_height(&self) -> Result<u64> {
		Ok(0)
	}

	fn execute(
		&self,
		_target_block: u64,
		_shard_id: ShardId,
		_task_id: TaskId,
		_cycle: TaskCycle,
		_task: TaskDescriptor,
		_block_num: i64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>> {
		future::ready(Ok([0u8; 64])).boxed()
	}
}

#[tokio::test]
async fn task_executor_smoke() -> Result<()> {
	let (client, backend) = {
		let builder = TestClientBuilder::with_default_backend();
		let backend = builder.backend();
		let (client, _) = builder.build_with_longest_chain();
		(Arc::new(client), backend)
	};

	let api = Arc::new(MockApi::default());

	let task_spawner = MockTask {};

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
