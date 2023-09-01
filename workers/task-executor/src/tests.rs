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
	AccountId, CycleStatus, Function, MembersApi, Network, PeerId, PublicKey, ShardId, ShardsApi,
	TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId, TaskPhase, TaskSpawner, TasksApi,
	TssPublicKey, TssSignature, TaskExecutor as OtherTaskExecutor
};
use sc_transaction_pool_api::{RejectAllTxPool, OffchainTransactionPoolFactory};
use sp_keystore::testing::MemoryKeystore;
use time_worker::tx_submitter::TransactionSubmitter;

lazy_static::lazy_static! {
	pub static ref TASK_STATUS: Arc<Mutex<HashMap<TaskId, bool>>> = Default::default();
}

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

#[derive(Clone, Default)]
struct MockApi {
	task_passed: bool,
}

sp_api::mock_impl_runtime_apis! {
	impl ShardsApi<Block> for MockApi{
		fn get_shards(account: &AccountId) -> Vec<ShardId> { vec![1] }
		fn get_shard_members(shard_id: ShardId) -> Vec<AccountId> { vec![] }
		fn get_shard_threshold(shard_id: ShardId) -> u16 { 1 }
		fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey) {}
	}
	impl TasksApi<Block> for MockApi{
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> { vec![TaskExecution::new(1,0,0, TaskPhase::default())] }
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor> { Some(TaskDescriptor{
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
		fn submit_task_hash(shard_id: ShardId, task_id: TaskId, hash: String) {}
		fn submit_task_result(task_id: TaskId, cycle: TaskCycle, status: CycleStatus) {
			TASK_STATUS.lock().unwrap().insert(task_id, status == CycleStatus::Success);
		}
		fn submit_task_error(shard_id: ShardId, error: TaskError) {
			TASK_STATUS.lock().unwrap().insert(task_id, false);
		}
	}
	impl MembersApi<Block> for MockApi{
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId> { None }
		fn submit_register_member(network: Network, public_key: PublicKey, peer_id: PeerId) {}
		fn submit_heartbeat(public_key: PublicKey) {}
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
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>> {
		if self.is_ok {
			future::ready(Ok([0u8; 64])).boxed()
		} else {
			future::ready(Err(anyhow::anyhow!("mock error"))).boxed()
		}
	}

	fn execute_write(
		&self,
		_function: Function,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>> {
		future::ready(Ok("".into())).boxed()
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

	let keystore = MemoryKeystore::new();
	let tx_submitter = TransactionSubmitter::new(
		false,
		keystore.into(),
		OffchainTransactionPoolFactory::new(RejectAllTxPool::default()),
		api.clone(),
	);

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
			tx_submitter: tx_submitter.clone()
		};

		let mut task_executor = TaskExecutor::new(params);
		let _ = task_executor.start_tasks(dummy_block_hash, 1, 1).await;

		loop {
			let Some(status) = TASK_STATUS.lock().unwrap().get(&1) else {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			};
			}
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
