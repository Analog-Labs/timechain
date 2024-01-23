use futures::channel::mpsc;
use futures::stream::{Stream, StreamExt};
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::{ApiExt, ApiRef, HeaderT, ProvideRuntimeApi};
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tc_subxt::AccountInterface;
use time_primitives::{
	AccountId, ApiResult, BlockHash, BlockNumber, BlockTimeApi, Commitment, MemberStatus,
	MembersApi, Network, NetworkId, NetworksApi, PeerId, PublicKey, Runtime, ShardId, ShardStatus,
	ShardsApi, SubmitResult, SubmitTransactionApi, TaskCycle, TaskDescriptor, TaskError,
	TaskExecution, TaskId, TaskResult, TasksApi, TssSignature, TxBuilder,
};

enum Tx {
	Commitment { shard_id: ShardId, commitment: Commitment, proof_of_knowledge: [u8; 65] },
	Ready { shard_id: ShardId },
	TaskHash { task_id: TaskId, cycle: TaskCycle, hash: Vec<u8> },
	TaskResult { task_id: TaskId, cycle: TaskCycle, result: TaskResult },
	TaskError { task_id: TaskId, cycle: TaskCycle, error: TaskError },
	TaskSignature { task_id: TaskId, signature: TssSignature },
	RegisterMember { network: Network, public_key: PublicKey, peer_id: PeerId, stake_amount: u128 },
	Heartbeat,
}

pub struct Substrate<B: Block, C, R, S> {
	_block: PhantomData<B>,
	register_extensions: bool,
	pool: OffchainTransactionPoolFactory<B>,
	client: Arc<C>,
	runtime: Arc<R>,
	subxt_client: S,
	tx: mpsc::UnboundedSender<Tx>,
}

impl<B, C, R, S> Substrate<B, C, R, S>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B> + BlockchainEvents<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: BlockTimeApi<B>
		+ NetworksApi<B>
		+ MembersApi<B>
		+ ShardsApi<B>
		+ TasksApi<B>
		+ SubmitTransactionApi<B>,
	S: AccountInterface + TxBuilder + Clone + Send + Sync + 'static,
{
	fn best_block(&self) -> B::Hash {
		self.client.info().best_hash
	}

	pub fn new(
		register_extensions: bool,
		pool: OffchainTransactionPoolFactory<B>,
		client: Arc<C>,
		runtime: Arc<R>,
		subxt_client: S,
	) -> Self {
		let (tx, rx) = mpsc::unbounded();
		let s = Self {
			_block: PhantomData,
			register_extensions,
			pool,
			client,
			runtime,
			subxt_client,
			tx,
		};
		tokio::task::spawn(s.clone().tx_submitter(rx));
		s
	}

	fn runtime_api(&self) -> ApiRef<'_, R::Api> {
		let mut runtime = self.runtime.runtime_api();
		if self.register_extensions {
			runtime.register_extension(self.pool.offchain_transaction_pool(self.best_block()));
		}
		runtime
	}

	fn submit_transaction(&self, tx: Tx) -> SubmitResult {
		self.tx.unbounded_send(tx).unwrap();
		Ok(Ok(()))
	}

	async fn tx_submitter(self, mut rx: mpsc::UnboundedReceiver<Tx>) {
		while let Some(tx) = rx.next().await {
			let tx = match tx {
				Tx::Commitment {
					shard_id,
					commitment,
					proof_of_knowledge,
				} => self.subxt_client.submit_commitment(shard_id, commitment, proof_of_knowledge),
				Tx::Ready { shard_id } => self.subxt_client.submit_online(shard_id),
				Tx::TaskHash { task_id, cycle, hash } => {
					self.subxt_client.submit_task_hash(task_id, cycle, hash)
				},
				Tx::TaskResult { task_id, cycle, result } => {
					self.subxt_client.submit_task_result(task_id, cycle, result)
				},
				Tx::TaskError { task_id, cycle, error } => {
					self.subxt_client.submit_task_error(task_id, cycle, error)
				},
				Tx::TaskSignature { task_id, signature } => {
					self.subxt_client.submit_task_signature(task_id, signature)
				},
				Tx::RegisterMember {
					network,
					public_key,
					peer_id,
					stake_amount,
				} => self.subxt_client.submit_register_member(
					network,
					public_key,
					peer_id,
					stake_amount,
				),
				Tx::Heartbeat => self.subxt_client.submit_heartbeat(),
			};
			let result = self.runtime_api().submit_transaction(self.best_block(), tx);
			match result {
				Ok(_) => self.subxt_client.increment_nonce(),
				Err(err) => {
					let nonce = self.subxt_client.nonce();
					tracing::error!(nonce, "{}", err);
				},
			}
		}
	}
}

impl<B: Block, C, R, S: Clone> Clone for Substrate<B, C, R, S> {
	fn clone(&self) -> Self {
		Self {
			_block: self._block,
			register_extensions: self.register_extensions,
			pool: self.pool.clone(),
			client: self.client.clone(),
			runtime: self.runtime.clone(),
			subxt_client: self.subxt_client.clone(),
			tx: self.tx.clone(),
		}
	}
}

impl<B, C, R, S> Runtime for Substrate<B, C, R, S>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B> + BlockchainEvents<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: BlockTimeApi<B>
		+ NetworksApi<B>
		+ MembersApi<B>
		+ ShardsApi<B>
		+ TasksApi<B>
		+ SubmitTransactionApi<B>,
	S: AccountInterface + TxBuilder + Clone + Send + Sync + 'static,
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

	fn get_shards(&self, block: BlockHash, account: &AccountId) -> ApiResult<Vec<ShardId>> {
		self.runtime_api().get_shards(block, account)
	}

	fn get_shard_members(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<Vec<(AccountId, MemberStatus)>> {
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
		commitment: Commitment,
		proof_of_knowledge: [u8; 65],
	) -> SubmitResult {
		self.submit_transaction(Tx::Commitment {
			shard_id,
			commitment,
			proof_of_knowledge,
		})
	}

	fn submit_online(&self, shard_id: ShardId) -> SubmitResult {
		self.submit_transaction(Tx::Ready { shard_id })
	}

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

	fn get_gateway(&self, network: Network) -> ApiResult<Option<Vec<u8>>> {
		self.runtime_api().get_gateway(self.best_block(), network)
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult {
		self.submit_transaction(Tx::TaskHash { task_id, cycle, hash })
	}

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		result: TaskResult,
	) -> SubmitResult {
		self.submit_transaction(Tx::TaskResult { task_id, cycle, result })
	}

	fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult {
		self.submit_transaction(Tx::TaskError { task_id, cycle, error })
	}

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> SubmitResult {
		self.submit_transaction(Tx::TaskSignature { task_id, signature })
	}

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

	fn get_min_stake(&self) -> ApiResult<u128> {
		self.runtime_api().get_min_stake(self.best_block())
	}

	fn submit_register_member(
		&self,
		network: Network,
		peer_id: PeerId,
		stake_amount: u128,
	) -> SubmitResult {
		let public_key = self.subxt_client.public_key();
		self.submit_transaction(Tx::RegisterMember {
			network,
			public_key,
			peer_id,
			stake_amount,
		})
	}

	fn submit_heartbeat(&self) -> SubmitResult {
		self.submit_transaction(Tx::Heartbeat)
	}

	fn get_network(&self, network_id: NetworkId) -> ApiResult<Option<(String, String)>> {
		self.runtime_api().get_network(self.best_block(), network_id)
	}
}
