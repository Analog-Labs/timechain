use futures::stream::{Stream, StreamExt};
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::{ApiExt, ApiRef, HeaderT, ProvideRuntimeApi};
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tc_subxt::SubxtClient;
use time_primitives::{
	AccountId, ApiResult, BlockHash, BlockNumber, BlockTimeApi, Commitment, Members, MembersApi,
	Network, PeerId, PublicKey, ShardId, ShardStatus, Shards, ShardsApi, SubmitResult,
	SubmitTransactionApi, TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId, TaskResult,
	Tasks, TasksApi, TssSignature,
};

pub struct Substrate<B: Block, C, R> {
	_block: PhantomData<B>,
	register_extensions: bool,
	pool: OffchainTransactionPoolFactory<B>,
	client: Arc<C>,
	runtime: Arc<R>,
	subxt_client: SubxtClient,
}

impl<B, C, R> Substrate<B, C, R>
where
	B: Block,
	C: HeaderBackend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: SubmitTransactionApi<B>,
{
	pub fn new(
		register_extensions: bool,
		pool: OffchainTransactionPoolFactory<B>,
		client: Arc<C>,
		runtime: Arc<R>,
		subxt_client: SubxtClient,
	) -> Self {
		Self {
			_block: PhantomData,
			register_extensions,
			pool,
			client,
			runtime,
			subxt_client,
		}
	}

	fn best_block(&self) -> B::Hash {
		self.client.info().best_hash
	}

	fn runtime_api(&self) -> ApiRef<'_, R::Api> {
		let mut runtime = self.runtime.runtime_api();
		if self.register_extensions {
			runtime.register_extension(self.pool.offchain_transaction_pool(self.best_block()));
		}
		runtime
	}

	fn submit_transaction(&self, tx: Vec<u8>) -> SubmitResult {
		let submit_res = self.runtime_api().submit_transaction(self.best_block(), tx);
		if submit_res.is_ok() {
			self.subxt_client.increment_nonce();
		}
		submit_res
	}
}

impl<B: Block, C, R> Clone for Substrate<B, C, R> {
	fn clone(&self) -> Self {
		Self {
			_block: self._block,
			register_extensions: self.register_extensions,
			pool: self.pool.clone(),
			client: self.client.clone(),
			runtime: self.runtime.clone(),
			subxt_client: self.subxt_client.clone(),
		}
	}
}

pub trait SubstrateClient {
	fn get_block_time_in_ms(&self) -> ApiResult<u64>;

	fn finality_notification_stream(
		&self,
	) -> Pin<Box<dyn Stream<Item = (BlockHash, BlockNumber)> + Send + 'static>>;

	fn public_key(&self) -> PublicKey;

	fn account_id(&self) -> AccountId;
}

impl<B, C, R> SubstrateClient for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash>,
	C: BlockchainEvents<B> + HeaderBackend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: BlockTimeApi<B> + SubmitTransactionApi<B>,
{
	fn get_block_time_in_ms(&self) -> ApiResult<u64> {
		self.runtime_api().get_block_time_in_msec(self.best_block())
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

	fn public_key(&self) -> PublicKey {
		self.subxt_client.public_key()
	}

	fn account_id(&self) -> AccountId {
		self.subxt_client.account_id()
	}
}

impl<B, C, R> Shards for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: ShardsApi<B> + SubmitTransactionApi<B>,
{
	fn get_shards(&self, block: BlockHash, account: &AccountId) -> ApiResult<Vec<ShardId>> {
		self.runtime_api().get_shards(block, account)
	}

	fn get_shard_members(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<Vec<AccountId>> {
		self.runtime_api().get_shard_members(block, shard_id)
	}

	fn get_shard_threshold(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<u16> {
		self.runtime_api().get_shard_threshold(block, shard_id)
	}

	fn get_shard_status(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<ShardStatus<BlockNumber>> {
		self.runtime_api().get_shard_status(block, shard_id)
	}

	fn get_shard_commitment(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<Commitment> {
		self.runtime_api().get_shard_commitment(block, shard_id)
	}

	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Vec<[u8; 33]>,
		proof_of_knowledge: [u8; 65],
	) -> SubmitResult {
		let tx = self.subxt_client.submit_commitment(shard_id, commitment, proof_of_knowledge);
		self.submit_transaction(tx)
	}

	fn submit_online(&self, shard_id: ShardId) -> SubmitResult {
		let tx = self.subxt_client.submit_ready(shard_id);
		self.submit_transaction(tx)
	}
}

impl<B, C, R> Tasks for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TasksApi<B> + SubmitTransactionApi<B>,
{
	fn get_shard_tasks(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<Vec<TaskExecution>> {
		self.runtime_api().get_shard_tasks(block, shard_id)
	}

	fn get_task(&self, block: BlockHash, task_id: TaskId) -> ApiResult<Option<TaskDescriptor>> {
		self.runtime_api().get_task(block, task_id)
	}

	fn get_task_signature(&self, task_id: TaskId) -> ApiResult<Option<TssSignature>> {
		self.runtime_api().get_task_signature(self.best_block(), task_id)
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult {
		let tx = self.subxt_client.submit_task_hash(task_id, cycle, hash);
		self.submit_transaction(tx)
	}

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult {
		let tx = self.subxt_client.submit_task_result(task_id, cycle, status);
		self.submit_transaction(tx)
	}

	fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult {
		let tx = self.subxt_client.submit_task_error(task_id, cycle, error);
		self.submit_transaction(tx)
	}

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> SubmitResult {
		let tx = self.subxt_client.submit_task_signature(task_id, signature);
		self.submit_transaction(tx)
	}
}

impl<B, C, R> Members for Substrate<B, C, R>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: MembersApi<B> + SubmitTransactionApi<B>,
{
	fn get_member_peer_id(
		&self,
		block: BlockHash,
		account: &AccountId,
	) -> ApiResult<Option<PeerId>> {
		self.runtime_api().get_member_peer_id(block, account)
	}

	fn get_heartbeat_timeout(&self) -> ApiResult<u64> {
		self.runtime_api().get_heartbeat_timeout(self.best_block())
	}

	fn submit_register_member(&self, network: Network, peer_id: PeerId) -> SubmitResult {
		let tx =
			self.subxt_client
				.register_member(network, self.subxt_client.public_key(), peer_id);
		self.submit_transaction(tx)
	}

	fn submit_heartbeat(&self) -> SubmitResult {
		let tx = self.subxt_client.submit_heartbeat();
		self.submit_transaction(tx)
	}
}
