use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::{ApiExt, ApiRef, ProvideRuntimeApi};
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::sync::Arc;
use time_primitives::{
	ApiResult, MembersApi, Network, PeerId, PublicKey, ShardId, ShardsApi, SubmitMembers,
	SubmitResult, SubmitShards, TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId,
	TaskResult, Tasks, TasksApi,
};

pub struct Substrate<B: Block, C, R> {
	_block: PhantomData<B>,
	kv: KeystorePtr,
	pool: OffchainTransactionPoolFactory<B>,
	register_extension: bool,
	client: Arc<C>,
	runtime: Arc<R>,
}

impl<B, C, R> Substrate<B, C, R>
where
	B: Block + 'static,
	C: HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
{
	pub fn new(
		register_extension: bool,
		kv: KeystorePtr,
		pool: OffchainTransactionPoolFactory<B>,
		client: Arc<C>,
		runtime: Arc<R>,
	) -> Self {
		Self {
			_block: PhantomData,
			kv,
			pool,
			register_extension,
			client,
			runtime,
		}
	}

	fn runtime_api(&self) -> (ApiRef<'_, R::Api>, B::Hash) {
		let block_hash = self.client.info().best_hash;
		let mut runtime = self.runtime.runtime_api();
		if self.register_extension {
			runtime.register_extension(KeystoreExt(self.kv.clone()));
			runtime.register_extension(self.pool.offchain_transaction_pool(block_hash));
		}
		(runtime, block_hash)
	}
}

impl<B: Block, C, R> Clone for Substrate<B, C, R> {
	fn clone(&self) -> Self {
		Self {
			_block: self._block,
			register_extension: self.register_extension,
			kv: self.kv.clone(),
			pool: self.pool.clone(),
			client: self.client.clone(),
			runtime: self.runtime.clone(),
		}
	}
}

impl<B, C, R> SubmitShards for Substrate<B, C, R>
where
	B: Block + 'static,
	C: HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: ShardsApi<B>,
{
	fn submit_commitment(
		&self,
		shard_id: ShardId,
		member: PublicKey,
		commitment: Vec<[u8; 33]>,
		proof_of_knowledge: [u8; 65],
	) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_commitment(block_hash, shard_id, member, commitment, proof_of_knowledge)
	}

	fn submit_online(&self, shard_id: ShardId, member: PublicKey) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_online(block_hash, shard_id, member)
	}
}

impl<B, C, R> Tasks<B> for Substrate<B, C, R>
where
	B: Block + 'static,
	C: HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
{
	fn get_shard_tasks(&self, block: B::Hash, shard_id: ShardId) -> ApiResult<Vec<TaskExecution>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shard_tasks(block, shard_id)
	}

	fn get_task(&self, block: B::Hash, task_id: TaskId) -> ApiResult<Option<TaskDescriptor>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_task(block, task_id)
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_task_hash(block_hash, task_id, cycle, hash)
	}

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_task_result(block_hash, task_id, cycle, status)
	}

	fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_task_error(block_hash, task_id, cycle, error)
	}
}

impl<B, C, R> SubmitMembers for Substrate<B, C, R>
where
	B: Block + 'static,
	C: HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B>,
{
	fn submit_register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_register_member(block_hash, network, public_key, peer_id)
	}

	fn submit_heartbeat(&self, public_key: PublicKey) -> SubmitResult {
		let (runtime_api, block_hash) = self.runtime_api();
		runtime_api.submit_heartbeat(block_hash, public_key)
	}
}
