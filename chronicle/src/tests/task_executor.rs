use crate::{TaskExecutor, TaskExecutorParams};
use anyhow::Result;
use futures::executor::block_on;
use futures::{future, FutureExt};
use sc_block_builder::BlockBuilderProvider;
use sc_network_test::{Block, TestClientBuilder, TestClientBuilderExt};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus::BlockOrigin;
use sp_runtime::AccountId32;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::{future::Future, pin::Pin};
use substrate_test_runtime_client::ClientBlockImportExt;
use time_primitives::{
	AccountId, Function, MembersApi, Network, PeerId, PublicKey, ShardId, ShardsApi, TaskCycle,
	TaskDescriptor, TaskError, TaskExecution, TaskExecutor as OtherTaskExecutor, TaskId, TaskPhase,
	TaskResult, TaskSpawner, TasksApi, TssPublicKey,
};

lazy_static::lazy_static! {
	pub static ref TASK_STATUS: Mutex<Vec<bool>> = Default::default();
}

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

#[derive(Clone, Default)]
struct MockApi;

sp_api::mock_impl_runtime_apis! {
	impl ShardsApi<Block> for MockApi{
		fn get_shards(_: &AccountId) -> Vec<ShardId> { vec![1] }
		fn get_shard_members(_: ShardId) -> Vec<AccountId> { vec![] }
		fn get_shard_threshold(_: ShardId) -> u16 { 1 }
		fn submit_tss_public_key(_: ShardId, _: TssPublicKey) {}
	}
	impl TasksApi<Block> for MockApi{
		fn get_shard_tasks(_: ShardId) -> Vec<TaskExecution<u32>> { vec![TaskExecution::new(1,0,0, TaskPhase::default())] }
		fn get_task(_: TaskId) -> Option<TaskDescriptor> { Some(TaskDescriptor{
				owner: AccountId32::new([0u8; 32]),
				network: Network::Ethereum,
				cycle: 0,
				function: Function::EvmViewCall {
					address: Default::default(),
					function_signature: Default::default(),
					input: Default::default(),
				},
				period: 0,
				start: 0,
				hash: "".to_string(),
			})
		}
		fn submit_task_hash(_: ShardId, _: TaskId, _: String) {}
		fn submit_task_result(_: TaskId, _: TaskCycle, _: TaskResult) {
			TASK_STATUS.lock().unwrap().push(true);
		}
		fn submit_task_error(_: TaskId, _: TaskCycle, _: TaskError) {
			TASK_STATUS.lock().unwrap().push(false);
		}
	}
	impl MembersApi<Block> for MockApi{
		fn get_member_peer_id(_: &AccountId) -> Option<PeerId> { None }
		fn get_heartbeat_timeout() -> u64 { 100 }
		fn submit_register_member(_: Network, _: PublicKey, _: PeerId) {}
		fn submit_heartbeat(_: PublicKey) {}
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

	fn execute_read(
		&self,
		_target_block: u64,
		_shard_id: ShardId,
		_task_id: TaskId,
		_cycle: TaskCycle,
		_function: Function,
		_hash: String,
		_block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		TASK_STATUS.lock().unwrap().push(self.is_ok);
		future::ready(Ok(())).boxed()
	}

	fn execute_write(
		&self,
		_: ShardId,
		_: TaskId,
		_: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		future::ready(Ok(())).boxed()
	}
}

#[tokio::test]
async fn task_executor_smoke() -> Result<()> {
	env_logger::try_init().ok();

	let (mut client, _) = {
		let builder = TestClientBuilder::with_default_backend();
		let backend = builder.backend();
		let (client, _) = builder.build_with_longest_chain();
		(Arc::new(client), backend)
	};
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
			runtime: api.clone(),
			task_spawner,
			network: Network::Ethereum,
			public_key: pubkey_from_bytes([i; 32]),
		};

		let task_executor = TaskExecutor::new(params);
		let _ = task_executor.start_tasks(dummy_block_hash, 1, 1).await;

		log::info!("waiting for result");
		loop {
			let Some(status) = TASK_STATUS.lock().unwrap().pop() else {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			};
			if is_task_ok {
				assert!(status);
				break;
			} else {
				assert!(!status);
				break;
			}
		}
	}
	Ok(())
}
