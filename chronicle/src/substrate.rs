use futures::stream::{Stream, StreamExt};
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::{ApiExt, ApiRef, HeaderT, ProvideRuntimeApi};
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use time_primitives::{
	AccountId, ApiResult, BlockHash, BlockNumber, BlockTimeApi, Commitment, Members, MembersApi,
	Network, PeerId, PublicKey, ShardId, ShardStatus, Shards, ShardsApi, SubmitResult, TaskCycle,
	TaskDescriptor, TaskError, TaskExecution, TaskId, TaskResult, Tasks, TasksApi,
};

pub struct Substrate<B: Block<Hash = BlockHash>, C, R> {
	_block: PhantomData<B>,
	kv: KeystorePtr,
	pool: OffchainTransactionPoolFactory<B>,
	register_extension: bool,
	client: Arc<C>,
	runtime: Arc<R>,
}

impl<B, C, R> Substrate<B, C, R>
where
	B: Block<Hash = BlockHash> + 'static,
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

impl<B: Block<Hash = BlockHash>, C, R> Clone for Substrate<B, C, R> {
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

pub trait SubstrateClient {
	fn get_block_time_in_ms(&self) -> ApiResult<u64>;

	fn finality_notification_stream(
		&self,
	) -> Pin<Box<dyn Stream<Item = (BlockHash, BlockNumber)> + Send + 'static>>;
}

impl<B, C, R> SubstrateClient for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash> + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: BlockTimeApi<B>,
{
	fn get_block_time_in_ms(&self) -> ApiResult<u64> {
		let (runtime_api, block) = self.runtime_api();
		runtime_api.get_block_time_in_msec(block)
	}

	fn finality_notification_stream(
		&self,
	) -> Pin<Box<dyn Stream<Item = (BlockHash, BlockNumber)> + Send + 'static>> {
		let stream = self.client.finality_notification_stream();
		stream
			.map(|notification| {
				let block_hash = notification.header.hash();
				let block_number = notification.header.number().to_string().parse().unwrap();
				(block_hash, block_number)
			})
			.boxed()
	}
}

impl<B, C, R> Shards for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash> + 'static,
	C: HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: ShardsApi<B>,
{
	fn get_shards(&self, block: BlockHash, account: &AccountId) -> ApiResult<Vec<ShardId>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shards(block, account)
	}

	fn get_shard_members(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<Vec<AccountId>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shard_members(block, shard_id)
	}

	fn get_shard_threshold(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<u16> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shard_threshold(block, shard_id)
	}

	fn get_shard_status(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<ShardStatus<BlockNumber>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shard_status(block, shard_id)
	}

	fn get_shard_commitment(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<Commitment> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shard_commitment(block, shard_id)
	}

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

impl<B, C, R> Tasks for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash> + 'static,
	C: HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
{
	fn get_shard_tasks(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<Vec<TaskExecution>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_shard_tasks(block, shard_id)
	}

	fn get_task(&self, block: BlockHash, task_id: TaskId) -> ApiResult<Option<TaskDescriptor>> {
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

impl<B, C, R> Members for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash> + 'static,
	C: HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B>,
{
	fn get_member_peer_id(
		&self,
		block: BlockHash,
		account: &AccountId,
	) -> ApiResult<Option<PeerId>> {
		let (runtime_api, _) = self.runtime_api();
		runtime_api.get_member_peer_id(block, account)
	}

	fn get_heartbeat_timeout(&self) -> ApiResult<u64> {
		let (runtime_api, block) = self.runtime_api();
		runtime_api.get_heartbeat_timeout(block)
	}

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
