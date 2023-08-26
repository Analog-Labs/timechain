use crate::{TaskExecutor, TaskExecutorParams};
use anyhow::Result;
use futures::executor::block_on;
use futures::{future, FutureExt};
use sc_block_builder::BlockBuilderProvider;
use sc_client_api::Backend;
use sc_network_test::{Block, TestClientBuilder, TestClientBuilderExt};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus::BlockOrigin;
use sp_runtime::AccountId32;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use std::{future::Future, pin::Pin};
use substrate_test_runtime_client::ClientBlockImportExt;
use time_primitives::{
	Function, Network, OcwPayload, PeerId, ShardId, TaskCycle, TaskDescriptor, TaskExecution,
	TaskId, TaskSpawner, TimeApi, TssSignature,
};

#[derive(Clone, Default)]
struct MockApi;

impl MockApi {}

sp_api::mock_impl_runtime_apis! {
	impl TimeApi<Block> for MockApi {
		fn get_shards(&self, _peer_id: PeerId) -> Vec<ShardId> {
			vec![1]
		}

		fn get_shard_tasks(&self, _shard_id: ShardId) -> Vec<TaskExecution> {
			vec![TaskExecution::new(1,0,0)]
		}

		fn get_task(&self, _task_id: TaskId) -> Option<TaskDescriptor> {
			Some(TaskDescriptor{
				owner: AccountId32::new([0u8; 32]),
				network: Network::Ethereum,
				cycle: 0,
				function: Function::EVMViewWithoutAbi {
					address: Default::default(),
					function_signature: Default::default(),
					input: Default::default(),
				},
				period: 0,
				start: 0,
				hash: "".to_string(),
			})
		}
	}
}

impl ProvideRuntimeApi<Block> for MockApi {
	type Api = Self;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.clone().into()
	}
}

struct MockTask {
	is_ok: bool,
}

impl MockTask {
	pub fn new(is_ok: bool) -> Self {
		Self { is_ok }
	}
}

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
		_block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>> {
		if self.is_ok {
			future::ready(Ok([0u8; 64])).boxed()
		} else {
			future::ready(Err(anyhow::anyhow!("mock error"))).boxed()
		}
	}
}

#[tokio::test]
async fn task_executor_smoke() -> Result<()> {
	let (mut client, backend) = {
		let builder = TestClientBuilder::with_default_backend();
		let backend = builder.backend();
		let (client, _) = builder.build_with_longest_chain();
		(Arc::new(client), backend)
	};
	let storage = backend.offchain_storage().unwrap();
	let api = Arc::new(MockApi);

	//import block
	let block = client.new_block(Default::default()).unwrap().build().unwrap().block;
	block_on(client.import(BlockOrigin::Own, block.clone())).unwrap();
	let dummy_block_hash = block.header.hash();

	for i in 0..3 {
		let is_task_ok = i % 2 == 0;
		let task_spawner = MockTask::new(is_task_ok);

		let params = TaskExecutorParams {
			_block: PhantomData,
			backend: backend.clone(),
			client: client.clone(),
			runtime: api.clone(),
			peer_id: [0u8; 32],
			task_spawner,
		};

		let mut task_executor = TaskExecutor::new(params);
		let _ = task_executor.start_tasks(dummy_block_hash, 1).await;

		loop {
			let Some(msg) = time_primitives::read_message(storage.clone()) else {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			};
			if is_task_ok {
				assert!(matches!(msg, OcwPayload::SubmitTaskResult { .. }));
				break;
			} else {
				assert!(matches!(msg, OcwPayload::SubmitTaskError { .. }));
				break;
			}
		}
	}
	Ok(())
}
