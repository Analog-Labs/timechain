use anyhow::Result;
use sc_network_test::Block;
use sp_api::{ApiRef, ProvideRuntimeApi};
use time_primitives::{PeerId, TaskCycle, ShardId, TaskId, TaskDescriptor, TimeApi};

#[derive(Clone, Default)]
struct MockApi;

impl MockApi {}

sp_api::mock_impl_runtime_apis! {
	impl TimeApi<Block> for MockApi {
		fn get_shards(&self, _peer_id: PeerId) -> Vec<ShardId> {
			vec![1]
		}

		fn get_shard_tasks(&self, _shard_id: ShardId) -> Vec<(TaskId, TaskCycle)> {
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
	Ok(())
}
