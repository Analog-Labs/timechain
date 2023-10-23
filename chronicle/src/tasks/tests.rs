use super::TaskSpawner;
use crate::substrate::Substrate;
use crate::{TaskExecutor, TaskExecutorParams};
use anyhow::Result;
use futures::executor::block_on;
use futures::{future, stream, FutureExt, Stream};
use sc_block_builder::BlockBuilderProvider;
use sc_network_test::{Block, TestClientBuilder, TestClientBuilderExt};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sc_transaction_pool_api::RejectAllTxPool;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus::BlockOrigin;
use sp_runtime::AccountId32;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::{future::Future, pin::Pin};
use substrate_test_runtime_client::ClientBlockImportExt;
use tc_subxt::AccountInterface;
use time_primitives::{
	AccountId, BlockNumber, Commitment, Function, MembersPayload, Network, PeerId,
	ProofOfKnowledge, PublicKey, ShardId, ShardsApi, ShardsPayload, TaskCycle, TaskDescriptor,
	TaskError, TaskExecution, TaskId, TaskPhase, TaskResult, TasksApi, TasksPayload, TssSignature,
	TxResult,
};
lazy_static::lazy_static! {
	pub static ref TASK_STATUS: Mutex<Vec<bool>> = Default::default();
}

#[derive(Clone, Default)]
struct MockApi;

sp_api::mock_impl_runtime_apis! {
	impl ShardsApi<Block> for MockApi{
		fn get_shards(_: &AccountId) -> Vec<ShardId> { vec![1] }
		fn get_shard_members(_: ShardId) -> Vec<AccountId> { vec![] }
		fn get_shard_threshold(_: ShardId) -> u16 { 1 }
	}

	impl TasksApi<Block> for MockApi{
		fn get_shard_tasks(_: ShardId) -> Vec<TaskExecution> { vec![TaskExecution::new(1,0,0, TaskPhase::default())] }
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
	}

	impl time_primitives::BlockTimeApi<Block> for MockApi{
		fn get_block_time_in_msec() -> u64{
			6000
		}
	}
	impl time_primitives::SubmitTransactionApi<Block> for MockApi {
		fn submit_transaction(_: Vec<u8>) -> TxResult {
			Ok(())
		}
	}
}

impl ProvideRuntimeApi<Block> for MockApi {
	type Api = Self;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.clone().into()
	}
}

#[derive(Clone)]
struct MockTask {
	is_ok: bool,
}

impl MockTask {
	pub fn new(is_ok: bool) -> Self {
		Self { is_ok }
	}
}

impl TaskSpawner for MockTask {
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		Box::pin(stream::iter(vec![1]))
	}

	fn execute_read(
		&self,
		_target_block: u64,
		_shard_id: ShardId,
		_task_id: TaskId,
		_cycle: TaskCycle,
		_function: Function,
		_hash: String,
		_block_num: BlockNumber,
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
#[derive(Clone)]
struct MockSubxt;

impl TasksPayload for MockSubxt {
	fn submit_task_hash(&self, _: TaskId, _: TaskCycle, _: Vec<u8>) -> Vec<u8> {
		vec![]
	}

	pub fn submit_task_signature(&self, _: TaskId, _: TssSignature) -> Vec<u8> {
		vec![]
	}

	fn submit_task_result(&self, _: TaskId, _: TaskCycle, _: TaskResult) -> Vec<u8> {
		TASK_STATUS.lock().unwrap().push(true);
		vec![]
	}

	fn submit_task_error(&self, _: TaskId, _: TaskCycle, _: TaskError) -> Vec<u8> {
		TASK_STATUS.lock().unwrap().push(false);
		vec![]
	}
}

impl ShardsPayload for MockSubxt {
	fn submit_commitment(&self, _: ShardId, _: Commitment, _: ProofOfKnowledge) -> Vec<u8> {
		vec![]
	}

	fn submit_online(&self, _: ShardId) -> Vec<u8> {
		vec![]
	}
}

impl MembersPayload for MockSubxt {
	fn submit_register_member(&self, _: Network, _: PublicKey, _: PeerId) -> Vec<u8> {
		vec![]
	}

	fn submit_heartbeat(&self) -> Vec<u8> {
		vec![]
	}
}

impl AccountInterface for MockSubxt {
	fn increment_nonce(&self) {}

	fn public_key(&self) -> PublicKey {
		let bytes = [0; 32];
		PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
	}

	fn account_id(&self) -> AccountId {
		let bytes = [0; 32];
		bytes.into()
	}

	fn nonce(&self) -> u64 {
		0
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

	let substrate = Substrate::new(
		false,
		OffchainTransactionPoolFactory::new(RejectAllTxPool::default()),
		client.clone(),
		api,
		MockSubxt {},
	);

	//import block
	let block = client.new_block(Default::default()).unwrap().build().unwrap().block;
	block_on(client.import(BlockOrigin::Own, block.clone())).unwrap();
	let dummy_block_hash = block.header.hash();

	for i in 0..3 {
		let is_task_ok = i % 2 == 0;
		let task_spawner = MockTask::new(is_task_ok);

		let params = TaskExecutorParams {
			task_spawner,
			network: Network::Ethereum,
			substrate: substrate.clone(),
		};

		let mut task_executor = TaskExecutor::new(params);
		task_executor.process_tasks(dummy_block_hash, 1, 1, 1).unwrap();

		tracing::info!("waiting for result");
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
