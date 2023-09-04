use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ApiError;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::sync::Arc;
use time_primitives::{
	MembersApi, Network, PeerId, PublicKey, ShardId, ShardsApi, SubmitMembers, SubmitShards,
	SubmitTasks, TaskCycle, TaskError, TaskId, TaskResult, TasksApi, TssPublicKey,
};

pub struct TransactionSubmitter<B, R>
where
	B: Block + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B>,
{
	_block: PhantomData<B>,
	kv: KeystorePtr,
	pool: OffchainTransactionPoolFactory<B>,
	register_extension: bool,
	runtime: Arc<R>,
}

impl<B, R> TransactionSubmitter<B, R>
where
	B: Block + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B>,
{
	pub fn new(
		register_extension: bool,
		kv: KeystorePtr,
		pool: OffchainTransactionPoolFactory<B>,
		runtime: Arc<R>,
	) -> Self {
		Self {
			_block: PhantomData,
			kv,
			pool,
			register_extension,
			runtime,
		}
	}
}

impl<B, R> Clone for TransactionSubmitter<B, R>
where
	B: Block + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B>,
{
	fn clone(&self) -> TransactionSubmitter<B, R> {
		TransactionSubmitter::new(
			self.register_extension,
			self.kv.clone(),
			self.pool.clone(),
			self.runtime.clone(),
		)
	}
}

impl<B, R> SubmitShards<B> for TransactionSubmitter<B, R>
where
	B: Block + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B>,
{
	fn submit_tss_pub_key(
		&self,
		block: <B as Block>::Hash,
		shard_id: ShardId,
		public_key: TssPublicKey,
	) -> Result<(), ApiError> {
		if self.register_extension {
			let mut runtime = self.runtime.runtime_api();
			runtime.register_extension(KeystoreExt(self.kv.clone()));
			runtime.register_extension(self.pool.offchain_transaction_pool(block));
			runtime.submit_tss_public_key(block, shard_id, public_key)
		} else {
			self.runtime.runtime_api().submit_tss_public_key(block, shard_id, public_key)
		}
	}
}

impl<B, R> SubmitTasks<B> for TransactionSubmitter<B, R>
where
	B: Block + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B>,
{
	fn submit_task_hash(
		&self,
		block: B::Hash,
		shard_id: ShardId,
		task_id: TaskId,
		hash: String,
	) -> Result<(), ApiError> {
		if self.register_extension {
			let mut runtime = self.runtime.runtime_api();
			runtime.register_extension(KeystoreExt(self.kv.clone()));
			runtime.register_extension(self.pool.offchain_transaction_pool(block));
			runtime.submit_task_hash(block, shard_id, task_id, hash)
		} else {
			self.runtime.runtime_api().submit_task_hash(block, shard_id, task_id, hash)
		}
	}
	fn submit_task_result(
		&self,
		block: B::Hash,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> Result<(), ApiError> {
		if self.register_extension {
			let mut runtime = self.runtime.runtime_api();
			runtime.register_extension(self.pool.offchain_transaction_pool(block));
			runtime.submit_task_result(block, task_id, cycle, status)
		} else {
			self.runtime.runtime_api().submit_task_result(block, task_id, cycle, status)
		}
	}
	fn submit_task_error(
		&self,
		block: B::Hash,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> Result<(), ApiError> {
		if self.register_extension {
			let mut runtime = self.runtime.runtime_api();
			runtime.register_extension(self.pool.offchain_transaction_pool(block));
			runtime.submit_task_error(block, task_id, cycle, error)
		} else {
			self.runtime.runtime_api().submit_task_error(block, task_id, cycle, error)
		}
	}
}

impl<B, R> SubmitMembers<B> for TransactionSubmitter<B, R>
where
	B: Block + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B>,
{
	fn submit_register_member(
		&self,
		block: B::Hash,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> Result<(), ApiError> {
		if self.register_extension {
			let mut runtime = self.runtime.runtime_api();
			runtime.register_extension(KeystoreExt(self.kv.clone()));
			runtime.register_extension(self.pool.offchain_transaction_pool(block));
			runtime.submit_register_member(block, network, public_key, peer_id)
		} else {
			self.runtime
				.runtime_api()
				.submit_register_member(block, network, public_key, peer_id)
		}
	}

	fn submit_heartbeat(&self, block: B::Hash, public_key: PublicKey) -> Result<(), ApiError> {
		if self.register_extension {
			let mut runtime = self.runtime.runtime_api();
			runtime.register_extension(KeystoreExt(self.kv.clone()));
			runtime.register_extension(self.pool.offchain_transaction_pool(block));
			runtime.submit_heartbeat(block, public_key)
		} else {
			self.runtime.runtime_api().submit_heartbeat(block, public_key)
		}
	}
}
