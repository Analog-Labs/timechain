use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use tc_subxt::SubxtClient;
use time_primitives::{
	AccountId, Address32, BatchId, BlockHash, BlockNumber, ChainName, ChainNetwork, Commitment,
	GatewayMessage, MemberStatus, NetworkId, PeerId, ProofOfKnowledge, PublicKey, ShardId,
	ShardStatus, Task, TaskId, TaskResult,
};

#[async_trait]
pub trait Runtime: Send + Sync + 'static {
	fn public_key(&self) -> &PublicKey;

	fn account_id(&self) -> &AccountId;

	async fn balance(&self, account: &AccountId, block: BlockHash) -> Result<u128>;

	fn block_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)>;

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)>;

	async fn is_registered(&self, block: BlockHash) -> Result<bool>;

	async fn get_network(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Option<(ChainName, ChainNetwork)>>;

	async fn get_member_peer_id(
		&self,
		account: &AccountId,
		block: BlockHash,
	) -> Result<Option<PeerId>>;

	async fn get_heartbeat_timeout(&self, block: BlockHash) -> Result<BlockNumber>;

	async fn is_heartbeat_submitted(&self, account: &AccountId, block: BlockHash) -> Result<bool>;

	async fn get_shards(&self, account: &AccountId, block: BlockHash) -> Result<Vec<ShardId>>;

	async fn get_shard_members(
		&self,
		shard_id: ShardId,
		block: BlockHash,
	) -> Result<Vec<(AccountId, MemberStatus)>>;

	async fn get_shard_threshold(&self, shard_id: ShardId, block: BlockHash) -> Result<u16>;

	async fn get_shard_status(&self, shard_id: ShardId, block: BlockHash) -> Result<ShardStatus>;

	async fn get_shard_commitment(
		&self,
		shard_id: ShardId,
		block: BlockHash,
	) -> Result<Option<Commitment>>;

	async fn get_shard_tasks(&self, shard_id: ShardId, block: BlockHash) -> Result<Vec<TaskId>>;

	async fn get_task(&self, task_id: TaskId, block: BlockHash) -> Result<Option<Task>>;

	async fn get_task_submitter(
		&self,
		task_id: TaskId,
		block: BlockHash,
	) -> Result<Option<PublicKey>>;

	async fn get_batch_message(
		&self,
		batch_id: BatchId,
		block: BlockHash,
	) -> Result<Option<GatewayMessage>>;

	async fn get_gateway(&self, network: NetworkId, block: BlockHash) -> Result<Option<Address32>>;

	async fn get_cctp_info(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Option<(Vec<Address32>, String)>>;

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

	async fn balance(&self, account: &AccountId, block: BlockHash) -> Result<u128> {
		self.balance(account, block).await
	}

	fn block_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.block_notification_stream()
	}

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.finality_notification_stream()
	}

	async fn is_registered(&self, block: BlockHash) -> Result<bool> {
		Ok(self.member_registered(self.account_id(), block).await?)
	}

	async fn get_network(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Option<(ChainName, ChainNetwork)>> {
		self.network_name(network, block).await
	}

	async fn get_member_peer_id(
		&self,
		account: &AccountId,
		block: BlockHash,
	) -> Result<Option<PeerId>> {
		self.member_peer_id(account, block).await
	}

	async fn get_heartbeat_timeout(&self, block: BlockHash) -> Result<BlockNumber> {
		self.heartbeat_timeout(block).await
	}

	async fn is_heartbeat_submitted(&self, account: &AccountId, block: BlockHash) -> Result<bool> {
		self.is_heartbeat_submitted(account, block).await
	}

	async fn get_shards(&self, account: &AccountId, block: BlockHash) -> Result<Vec<ShardId>> {
		self.member_shards(account, block).await
	}

	async fn get_shard_members(
		&self,
		shard: ShardId,
		block: BlockHash,
	) -> Result<Vec<(AccountId, MemberStatus)>> {
		self.shard_members(shard, block).await
	}

	async fn get_shard_threshold(&self, shard: ShardId, block: BlockHash) -> Result<u16> {
		self.shard_threshold(shard, block).await
	}

	async fn get_shard_status(&self, shard: ShardId, block: BlockHash) -> Result<ShardStatus> {
		self.shard_status(shard, block).await
	}

	async fn get_shard_commitment(
		&self,
		shard: ShardId,
		block: BlockHash,
	) -> Result<Option<Commitment>> {
		self.shard_commitment(shard, block).await
	}

	async fn get_shard_tasks(&self, shard: ShardId, block: BlockHash) -> Result<Vec<TaskId>> {
		self.assigned_tasks(shard, block).await
	}

	async fn get_task(&self, task_id: TaskId, block: BlockHash) -> Result<Option<Task>> {
		self.task(task_id, block).await
	}

	async fn get_batch_message(
		&self,
		batch: BatchId,
		block: BlockHash,
	) -> Result<Option<GatewayMessage>> {
		self.batch_message(batch, block).await
	}

	async fn get_task_submitter(
		&self,
		task_id: TaskId,
		block: BlockHash,
	) -> Result<Option<PublicKey>> {
		self.task_submitter(task_id, block).await
	}

	async fn get_gateway(&self, network: NetworkId, block: BlockHash) -> Result<Option<Address32>> {
		self.network_gateway(network, block).await
	}

	async fn get_cctp_info(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Option<(Vec<Address32>, String)>> {
		let contracts_opt = self.get_cctp_contracts(network, block).await?;
		let url_opt = self.get_cctp_url(network, block).await?;

		match (contracts_opt, url_opt) {
			(Some(contracts), Some(url)) => {
				let contracts = contracts.0.to_vec();
				let url = String::from_utf8(url.0.to_vec())?;
				Ok(Some((contracts, url)))
			},
			_ => Ok(None),
		}
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
