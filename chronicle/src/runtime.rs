use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use tc_subxt::SubxtClient;
use time_primitives::{
	AccountId, Balance, BatchId, BlockHash, BlockNumber, ChainName, ChainNetwork, Commitment,
	Gateway, GatewayMessage, MemberStatus, NetworkId, PeerId, ProofOfKnowledge, PublicKey, ShardId,
	ShardStatus, Task, TaskId, TaskResult,
};

#[async_trait]
pub trait Runtime: Send + Sync + 'static {
	fn public_key(&self) -> &PublicKey;

	fn account_id(&self) -> &AccountId;

	async fn balance(&self, account: &AccountId) -> Result<u128>;

	fn block_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)>;

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)>;

	async fn is_registered(&self) -> Result<bool> {
		Ok(true)
	}

	async fn get_network(&self, network: NetworkId) -> Result<Option<(ChainName, ChainNetwork)>>;

	async fn get_member_peer_id(&self, account: &AccountId) -> Result<Option<PeerId>>;

	async fn get_heartbeat_timeout(&self) -> Result<BlockNumber>;

	async fn get_min_stake(&self) -> Result<Balance>;

	async fn get_shards(&self, account: &AccountId) -> Result<Vec<ShardId>>;

	async fn get_shard_members(&self, shard_id: ShardId) -> Result<Vec<(AccountId, MemberStatus)>>;

	async fn get_shard_threshold(&self, shard_id: ShardId) -> Result<u16>;

	async fn get_shard_status(&self, shard_id: ShardId) -> Result<ShardStatus>;

	async fn get_shard_commitment(&self, shard_id: ShardId) -> Result<Option<Commitment>>;

	async fn get_shard_tasks(&self, shard_id: ShardId) -> Result<Vec<TaskId>>;

	async fn get_task(&self, task_id: TaskId) -> Result<Option<Task>>;

	async fn get_task_submitter(&self, task_id: TaskId) -> Result<Option<PublicKey>>;

	async fn get_batch_message(&self, batch_id: BatchId) -> Result<Option<GatewayMessage>>;

	async fn get_gateway(&self, network: NetworkId) -> Result<Option<Gateway>>;

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<()>;

	async fn submit_unregister_member(&self) -> Result<()>;

	async fn submit_heartbeat(&self) -> Result<()>;

	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> Result<()>;

	async fn submit_online(&self, shard_id: ShardId) -> Result<()>;

	async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()>;
}

#[async_trait]
impl Runtime for SubxtClient {
	fn public_key(&self) -> &PublicKey {
		self.public_key()
	}

	fn account_id(&self) -> &AccountId {
		self.account_id()
	}

	async fn balance(&self, account: &AccountId) -> Result<u128> {
		self.balance(account).await
	}

	fn block_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.block_notification_stream()
	}

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.finality_notification_stream()
	}

	async fn is_registered(&self) -> Result<bool> {
		Ok(self.member_network(self.account_id()).await?.is_some())
	}

	async fn get_network(&self, network: NetworkId) -> Result<Option<(ChainName, ChainNetwork)>> {
		self.network_name(network).await
	}

	async fn get_member_peer_id(&self, account: &AccountId) -> Result<Option<PeerId>> {
		self.member_peer_id(account).await
	}

	async fn get_heartbeat_timeout(&self) -> Result<BlockNumber> {
		self.heartbeat_timeout().await
	}

	async fn get_min_stake(&self) -> Result<Balance> {
		self.min_stake().await
	}

	async fn get_shards(&self, account: &AccountId) -> Result<Vec<ShardId>> {
		self.member_shards(account).await
	}

	async fn get_shard_members(&self, shard: ShardId) -> Result<Vec<(AccountId, MemberStatus)>> {
		self.shard_members(shard).await
	}

	async fn get_shard_threshold(&self, shard: ShardId) -> Result<u16> {
		self.shard_threshold(shard).await
	}

	async fn get_shard_status(&self, shard: ShardId) -> Result<ShardStatus> {
		self.shard_status(shard).await
	}

	async fn get_shard_commitment(&self, shard: ShardId) -> Result<Option<Commitment>> {
		self.shard_commitment(shard).await
	}

	async fn get_shard_tasks(&self, shard: ShardId) -> Result<Vec<TaskId>> {
		self.assigned_tasks(shard).await
	}

	async fn get_task(&self, task_id: TaskId) -> Result<Option<Task>> {
		self.task(task_id).await
	}

	async fn get_batch_message(&self, batch: BatchId) -> Result<Option<GatewayMessage>> {
		self.batch_message(batch).await
	}

	async fn get_task_submitter(&self, task_id: TaskId) -> Result<Option<PublicKey>> {
		self.task_submitter(task_id).await
	}

	async fn get_gateway(&self, network: NetworkId) -> Result<Option<Gateway>> {
		self.network_gateway(network).await
	}

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<()> {
		self.register_member(network, peer_id, stake_amount).await
	}

	async fn submit_unregister_member(&self) -> Result<()> {
		self.unregister_member().await
	}

	async fn submit_heartbeat(&self) -> Result<()> {
		self.submit_heartbeat().await
	}

	async fn submit_commitment(
		&self,
		shard: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> Result<()> {
		self.submit_commitment(shard, commitment, proof_of_knowledge).await
	}

	async fn submit_online(&self, shard: ShardId) -> Result<()> {
		self.submit_online(shard).await
	}

	async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
		self.submit_task_result(task_id, result).await
	}
}
