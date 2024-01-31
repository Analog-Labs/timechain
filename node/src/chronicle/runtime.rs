use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{self, Stream, StreamExt};
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::{ApiExt, ApiRef, HeaderT, ProvideRuntimeApi};
use sp_core::H256;
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tc_subxt::{OnlineClient, PolkadotConfig, StreamOfResults, SubxtClient, TxProgress};
use time_primitives::{
	AccountId, BlockHash, BlockNumber, BlockTimeApi, Commitment, MemberStatus, MembersApi,
	NetworkId, NetworksApi, PeerId, ProofOfKnowledge, PublicKey, Runtime, ShardId, ShardStatus,
	ShardsApi, SubmitTransactionApi, TaskCycle, TaskDescriptor, TaskDescriptorParams, TaskError,
	TaskExecution, TaskId, TaskResult, TasksApi, TssSignature, TxSubmitter,
};

pub struct Substrate<B: Block, C, R> {
	_block: PhantomData<B>,
	client: Arc<C>,
	runtime: Arc<R>,
	subxt_client: SubxtClient,
}

impl<B, C, R> Substrate<B, C, R>
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
{
	fn best_block(&self) -> B::Hash {
		self.client.info().best_hash
	}

	pub fn new(client: Arc<C>, runtime: Arc<R>, subxt_client: SubxtClient) -> Self {
		Self {
			_block: PhantomData,
			client,
			runtime,
			subxt_client,
		}
	}

	fn runtime_api(&self) -> ApiRef<'_, R::Api> {
		self.runtime.runtime_api()
	}
}

impl<B: Block, C, R> Clone for Substrate<B, C, R> {
	fn clone(&self) -> Self {
		Self {
			_block: self._block,
			client: self.client.clone(),
			runtime: self.runtime.clone(),
			subxt_client: self.subxt_client.clone(),
		}
	}
}

#[async_trait]
impl<B, C, R> Runtime for Substrate<B, C, R>
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
{
	fn public_key(&self) -> &PublicKey {
		self.subxt_client.public_key()
	}

	fn account_id(&self) -> &AccountId {
		self.subxt_client.account_id()
	}

	async fn get_block_time_in_ms(&self) -> Result<u64> {
		Ok(self.runtime_api().get_block_time_in_msec(self.best_block())?)
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

	async fn get_shards(&self, block: BlockHash, account: &AccountId) -> Result<Vec<ShardId>> {
		Ok(self.runtime_api().get_shards(block, account)?)
	}

	async fn get_shard_members(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<(AccountId, MemberStatus)>> {
		Ok(self.runtime_api().get_shard_members(block, shard_id)?)
	}

	async fn get_shard_threshold(&self, block: BlockHash, shard_id: ShardId) -> Result<u16> {
		Ok(self.runtime_api().get_shard_threshold(block, shard_id)?)
	}

	async fn get_shard_status(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<ShardStatus<BlockNumber>> {
		Ok(self.runtime_api().get_shard_status(block, shard_id)?)
	}

	async fn get_shard_commitment(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<Commitment> {
		Ok(self.runtime_api().get_shard_commitment(block, shard_id)?)
	}

	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,

		proof_of_knowledge: ProofOfKnowledge,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client
			.submit_commitment(shard_id, commitment, proof_of_knowledge)
			.await
	}

	async fn submit_online(
		&self,
		shard_id: ShardId,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_online(shard_id).await
	}

	async fn get_shard_tasks(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<TaskExecution>> {
		Ok(self.runtime_api().get_shard_tasks(block, shard_id)?)
	}

	async fn get_task(&self, block: BlockHash, task_id: TaskId) -> Result<Option<TaskDescriptor>> {
		Ok(self.runtime_api().get_task(block, task_id)?)
	}

	async fn get_task_signature(&self, task_id: TaskId) -> Result<Option<TssSignature>> {
		Ok(self.runtime_api().get_task_signature(self.best_block(), task_id)?)
	}

	async fn get_gateway(&self, network: NetworkId) -> Result<Option<Vec<u8>>> {
		Ok(self.runtime_api().get_gateway(self.best_block(), network)?)
	}

	async fn submit_task_hash(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		hash: Vec<u8>,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_task_hash(task_id, cycle, hash).await
	}

	async fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,

		result: TaskResult,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_task_result(task_id, cycle, result).await
	}

	async fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_task_error(task_id, cycle, error).await
	}

	async fn submit_task_signature(
		&self,
		task_id: TaskId,
		signature: TssSignature,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_task_signature(task_id, signature).await
	}

	async fn get_member_peer_id(
		&self,
		block: BlockHash,
		account: &AccountId,
	) -> Result<Option<PeerId>> {
		Ok(self.runtime_api().get_member_peer_id(block, account)?)
	}

	async fn get_heartbeat_timeout(&self) -> Result<u64> {
		Ok(self.runtime_api().get_heartbeat_timeout(self.best_block())?)
	}

	async fn get_min_stake(&self) -> Result<u128> {
		Ok(self.runtime_api().get_min_stake(self.best_block())?)
	}

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_register_member(network, peer_id, stake_amount).await
	}

	async fn submit_heartbeat(
		&self,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.submit_heartbeat().await
	}

	async fn get_network(&self, network_id: NetworkId) -> Result<Option<(String, String)>> {
		Ok(self.runtime_api().get_network(self.best_block(), network_id)?)
	}

	async fn create_task(
		&self,
		task: TaskDescriptorParams,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.create_task(task).await
	}

	async fn insert_gateway(
		&self,
		shard_id: ShardId,
		address: Vec<u8>,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.subxt_client.insert_gateway(shard_id, address).await
	}
}

pub struct SubstrateTxSubmitter<B: Block, C, R> {
	_marker: PhantomData<B>,
	client: Arc<C>,
	pool: OffchainTransactionPoolFactory<B>,
	runtime: Arc<R>,
	tx_client: OnlineClient<PolkadotConfig>,
}

impl<B: Block, C, R> Clone for SubstrateTxSubmitter<B, C, R> {
	fn clone(&self) -> Self {
		Self {
			_marker: self._marker,
			client: self.client.clone(),
			pool: self.pool.clone(),
			runtime: self.runtime.clone(),
			tx_client: self.tx_client.clone(),
		}
	}
}

impl<B, C, R> SubstrateTxSubmitter<B, C, R>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B> + BlockchainEvents<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: SubmitTransactionApi<B>,
{
	pub fn new(
		pool: OffchainTransactionPoolFactory<B>,
		client: Arc<C>,
		runtime: Arc<R>,
		tx_client: OnlineClient<PolkadotConfig>,
	) -> Self {
		Self {
			_marker: PhantomData,
			client,
			pool,
			runtime,
			tx_client,
		}
	}

	fn best_block(&self) -> B::Hash {
		self.client.info().best_hash
	}

	fn runtime_api(&self) -> ApiRef<'_, R::Api> {
		let mut runtime = self.runtime.runtime_api();
		runtime.register_extension(self.pool.offchain_transaction_pool(self.best_block()));
		runtime
	}
}

#[async_trait]
impl<B, C, R> TxSubmitter for SubstrateTxSubmitter<B, C, R>
where
	B: Block<Hash = BlockHash>,
	C: HeaderBackend<B> + BlockchainEvents<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: SubmitTransactionApi<B>,
{
	async fn submit(
		&self,
		tx: Vec<u8>,
	) -> Result<TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>>> {
		self.runtime_api()
			.submit_transaction(self.best_block(), tx)
			.map_err(|_| anyhow::anyhow!("Error submitting transaction to runtime"))?
			.map_err(|_| anyhow::anyhow!("Error submitting transaction onchain"))?;
		let dummy_hash = H256::repeat_byte(0x01);
		let dummy_stream = stream::iter(vec![]);
		let empty_progress: TxProgress<PolkadotConfig, OnlineClient<PolkadotConfig>> =
			TxProgress::new(
				StreamOfResults::new(Box::pin(dummy_stream)),
				self.tx_client.clone(),
				dummy_hash,
			);
		Ok(empty_progress)
	}
}
