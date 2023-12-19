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
	AccountId, BlockNumber, Commitment, Function, MemberStatus, MembersPayload, Network, PeerId,
	ProofOfKnowledge, PublicKey, ShardId, ShardStatus, ShardsApi, ShardsPayload, TaskCycle,
	TaskDescriptor, TaskError, TaskExecution, TaskId, TaskPhase, TaskResult, TasksApi,
	TasksPayload, TssSignature, TxResult,
};
lazy_static::lazy_static! {
	pub static ref TASK_STATUS: Mutex<Vec<bool>> = Default::default();
}
lazy_static::lazy_static! {
	pub static ref GMP_TASK_PHASE: Mutex<TaskPhase> = Mutex::new(TaskPhase::Sign);
}

fn get_mock_pubkey() -> PublicKey {
	let bytes = [0; 32];
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

#[derive(Clone, Default)]
struct MockApi;

sp_api::mock_impl_runtime_apis! {
	impl ShardsApi<Block> for MockApi{
		fn get_shards(_: &AccountId) -> Vec<ShardId> { vec![1] }
		fn get_shard_members(_: ShardId) -> Vec<(AccountId, MemberStatus)> { vec![] }
		fn get_shard_threshold(_: ShardId) -> u16 { 1 }
		fn get_shard_status(_: ShardId) -> ShardStatus<BlockNumber>{
			ShardStatus::Offline
		}
		fn get_shard_commitment(_: ShardId) -> Commitment{
			vec![[0; 33]]
		}
	}

	impl TasksApi<Block> for MockApi{
		// shard 1 returns a viewcall task, shard 2 returns gmp task
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			if shard_id == 1 {
				vec![TaskExecution::new(1,1,0, self.get_task_phase(Default::default(), 1).unwrap())]
			}else {
				vec![TaskExecution::new(2,1,0, self.get_task_phase(Default::default(), 2).unwrap())]
			}
		}
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor> {
			if task_id == 1 {
				Some(TaskDescriptor{
				owner: Some(AccountId32::new([0u8; 32])),
				network: Network::Ethereum,
				cycle: 1,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				period: 0,
				start: 0,
				timegraph: None,
				})
			} else {
				Some(TaskDescriptor{
				owner: Some(AccountId32::new([0u8; 32])),
				network: Network::Ethereum,
				cycle: 1,
				function: Function::SendMessage {
					address: Default::default(),
					gas_limit: Default::default(),
					salt: Default::default(),
					payload: Default::default(),
				},
				period: 0,
				start: 0,
				timegraph: None,
				})
			}
		}
		fn get_task_signature(_: TaskId) -> Option<TssSignature>{
			Some([0; 64])
		}
		fn get_task_cycle(_: TaskId) -> TaskCycle{
			0
		}
		fn get_task_phase(task_id: TaskId) -> TaskPhase{
			if task_id == 1 {
				TaskPhase::Read(None)
			}else {
				GMP_TASK_PHASE.lock().unwrap().clone()
			}
		}
		fn get_task_results(_: TaskId, _: Option<TaskCycle>) -> Vec<(TaskCycle, TaskResult)>{
			vec![]
		}
		fn get_task_shard(_: TaskId) -> Option<ShardId>{
			None
		}
		fn get_gateway(_: Network) -> Option<Vec<u8>> {
			Some([0; 20].into())
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
		shard_id: ShardId,
		_task_id: TaskId,
		_cycle: TaskCycle,
		function: Function,
		_hash: Option<[u8; 32]>,
		_block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		if shard_id == 1 {
			TASK_STATUS.lock().unwrap().push(self.is_ok);
		}

		if shard_id == 2 {
			let tx_receipt: Vec<u8> = [0; 32].into();
			let is_receipt_valid =
				matches!(function, Function::EvmTxReceipt { tx } if tx == tx_receipt);
			TASK_STATUS.lock().unwrap().push(is_receipt_valid);
		}

		future::ready(Ok(())).boxed()
	}

	fn execute_sign(
		&self,
		_: ShardId,
		_: TaskId,
		_: TaskCycle,
		_: [u8; 32],
		_: u32,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		*GMP_TASK_PHASE.lock().unwrap() = TaskPhase::Write(get_mock_pubkey());
		future::ready(Ok(())).boxed()
	}

	fn execute_write(
		&self,
		_: ShardId,
		_: TaskId,
		_: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		*GMP_TASK_PHASE.lock().unwrap() = TaskPhase::Read(Some([0; 32].into()));
		future::ready(Ok(())).boxed()
	}
}
#[derive(Clone)]
struct MockSubxt;

impl TasksPayload for MockSubxt {
	fn submit_task_hash(&self, _: TaskId, _: TaskCycle, _: Vec<u8>) -> Vec<u8> {
		vec![]
	}

	fn submit_task_signature(&self, _: TaskId, _: TssSignature) -> Vec<u8> {
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
		get_mock_pubkey()
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
#[tokio::test]
async fn gmp_smoke() -> Result<()> {
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

	let task_spawner = MockTask::new(true);

	let params = TaskExecutorParams {
		task_spawner,
		network: Network::Ethereum,
		substrate: substrate.clone(),
	};

	let mut task_executor = TaskExecutor::new(params);

	loop {
		task_executor.process_tasks(dummy_block_hash, 1, 2, 1).unwrap();
		tracing::info!("Watching for result");
		let Some(status) = TASK_STATUS.lock().unwrap().pop() else {
			tokio::time::sleep(Duration::from_secs(5)).await;
			tracing::info!("task phase {:?}", GMP_TASK_PHASE.lock().unwrap());
			continue;
		};
		assert!(status);
		break;
	}
	Ok(())
}
