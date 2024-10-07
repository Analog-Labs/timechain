use crate::runtime::Runtime;
use anyhow::Result;
use futures::stream::BoxStream;
use futures::{stream, StreamExt};
use schnorr_evm::k256::ProjectivePoint;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time_primitives::traits::IdentifyAccount;
use time_primitives::{
	sr25519, AccountId, Balance, BatchId, BlockHash, BlockNumber, ChainName, ChainNetwork,
	Commitment, Gateway, GatewayMessage, MemberStatus, NetworkId, PeerId, ProofOfKnowledge,
	PublicKey, ShardId, ShardStatus, Task, TaskId, TaskResult,
};
use tokio::time::Duration;
use tss::{sum_commitments, VerifiableSecretSharingCommitment, VerifyingKey};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MockNetwork {
	pub chain_name: ChainName,
	pub chain_network: ChainNetwork,
}

impl MockNetwork {
	pub fn new(chain_name: ChainName, chain_network: ChainNetwork) -> Self {
		Self { chain_name, chain_network }
	}
}

#[derive(Clone, Debug)]
pub struct MockShard {
	pub members: Vec<(AccountId, MemberStatus)>,
	pub threshold: u16,
	pub commitments: Vec<Commitment>,
	pub online: usize,
}

impl MockShard {
	pub fn new(members: Vec<AccountId>, threshold: u16) -> Self {
		Self {
			members: members.into_iter().map(|member| (member, MemberStatus::Ready)).collect(),
			threshold,
			commitments: vec![],
			online: 0,
		}
	}
}

#[derive(Clone, Debug)]
pub struct MockTask {
	pub task: Task,
	pub result: Option<TaskResult>,
	pub submitter: Option<PublicKey>,
}

impl MockTask {
	pub fn new(task: Task) -> Self {
		Self {
			task,
			result: None,
			submitter: None,
		}
	}
}

pub struct MockBatch {
	pub message: GatewayMessage,
}

type Map<K, V> = Arc<Mutex<HashMap<K, V>>>;

#[derive(Clone, Default)]
pub struct Mock {
	public_key: Option<PublicKey>,
	account_id: Option<AccountId>,
	networks: Map<NetworkId, MockNetwork>,
	members: Map<NetworkId, Vec<(PublicKey, PeerId)>>,
	shards: Map<ShardId, MockShard>,
	tasks: Map<TaskId, MockTask>,
	assigned_tasks: Map<TaskId, ShardId>,
	batches: Map<BatchId, MockBatch>,
}

impl Mock {
	pub fn instance(&self, id: u8) -> Self {
		let mut mock = self.clone();
		let public_key = PublicKey::Sr25519(sr25519::Public::from_raw([id; 32]));
		mock.public_key = Some(public_key.clone());
		mock.account_id = Some(public_key.into_account());
		mock
	}

	pub fn create_network(&self, chain_name: ChainName, chain_network: ChainNetwork) -> NetworkId {
		let mock_network = MockNetwork::new(chain_name, chain_network);
		let mut networks = self.networks.lock().unwrap();
		if let Some(existing_id) =
			networks
				.iter()
				.find_map(|(key, val)| if *val == mock_network { Some(key) } else { None })
		{
			return *existing_id;
		}
		let network_id = networks.len() as _;
		networks.insert(network_id, mock_network);
		network_id
	}

	pub fn create_shard(&self, members: Vec<AccountId>, threshold: u16) -> ShardId {
		let mut shards = self.shards.lock().unwrap();
		let shard_id = shards.len() as _;
		shards.insert(shard_id, MockShard::new(members, threshold));
		shard_id
	}

	#[allow(unused)]
	pub fn create_online_shard(&self, members: Vec<AccountId>, threshold: u16) -> ShardId {
		let shard_id = self.create_shard(members, threshold);
		let mut shards = self.shards.lock().unwrap();
		let shard = shards.get_mut(&shard_id).unwrap();
		let public_key = VerifyingKey::new(ProjectivePoint::GENERATOR).to_bytes().unwrap();
		shard.commitments = vec![vec![public_key; threshold as _]; shard.members.len()];
		shard.online = shard.members.len();
		shard_id
	}

	pub fn create_task(&self, task: Task) -> TaskId {
		let mut tasks = self.tasks.lock().unwrap();
		let task_id = tasks.len() as _;
		tasks.insert(task_id, MockTask::new(task));
		task_id
	}

	pub fn assign_task(&self, task_id: TaskId, shard_id: ShardId) {
		let mut assigned_tasks = self.assigned_tasks.lock().unwrap();
		assigned_tasks.insert(task_id, shard_id);
	}

	pub fn members(&self, network_id: NetworkId) -> Vec<(PublicKey, PeerId)> {
		let members = self.members.lock().unwrap();
		members.get(&network_id).cloned().unwrap_or_default()
	}

	#[allow(unused)]
	pub fn shard(&self, shard_id: ShardId) -> Option<MockShard> {
		let shards = self.shards.lock().unwrap();
		shards.get(&shard_id).cloned()
	}

	pub fn task(&self, task_id: TaskId) -> Option<MockTask> {
		let tasks = self.tasks.lock().unwrap();
		tasks.get(&task_id).cloned()
	}
}

#[async_trait::async_trait]
impl Runtime for Mock {
	fn public_key(&self) -> &PublicKey {
		self.public_key.as_ref().unwrap()
	}

	fn account_id(&self) -> &AccountId {
		self.account_id.as_ref().unwrap()
	}

	async fn balance(&self, _account: &AccountId) -> Result<u128> {
		Ok(100_000)
	}

	fn block_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		stream::iter(std::iter::successors(Some(([0; 32].into(), 0)), |(_, n)| {
			let n = n + 1;
			Some(([n as _; 32].into(), n))
		}))
		.then(|e| async move {
			tokio::time::sleep(Duration::from_secs(1)).await;
			e
		})
		.boxed()
	}

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		self.block_notification_stream()
			.then(|e| async move {
				tokio::time::sleep(Duration::from_secs(1)).await;
				e
			})
			.boxed()
	}

	async fn get_network(&self, network: NetworkId) -> Result<Option<(ChainName, ChainNetwork)>> {
		Ok(self
			.networks
			.lock()
			.unwrap()
			.get(&network)
			.map(|network| (network.chain_name.clone(), network.chain_network.clone())))
	}

	async fn get_member_peer_id(&self, account: &AccountId) -> Result<Option<PeerId>> {
		let members = self.members.lock().unwrap();
		Ok(members
			.iter()
			.flat_map(|(_, members)| members.iter())
			.find(|(acc, _)| &acc.clone().into_account() == account)
			.map(|(_, peer_id)| *peer_id))
	}

	async fn get_heartbeat_timeout(&self) -> Result<BlockNumber> {
		Ok(1000)
	}

	async fn get_min_stake(&self) -> Result<Balance> {
		Ok(0)
	}

	async fn get_shards(&self, account: &AccountId) -> Result<Vec<ShardId>> {
		let shards = self.shards.lock().unwrap();
		let shards = shards
			.iter()
			.filter(|(_, shard)| shard.members.iter().any(|(acc, _)| acc == account))
			.map(|(shard, _)| *shard)
			.collect();
		Ok(shards)
	}

	async fn get_shard_members(&self, shard_id: ShardId) -> Result<Vec<(AccountId, MemberStatus)>> {
		let shards = self.shards.lock().unwrap();
		let members = shards.get(&shard_id).map(|shard| shard.members.clone()).unwrap_or_default();
		Ok(members)
	}

	async fn get_shard_threshold(&self, shard_id: ShardId) -> Result<u16> {
		let shards = self.shards.lock().unwrap();
		let threshold = shards.get(&shard_id).map(|shard| shard.threshold).unwrap_or_default();
		Ok(threshold)
	}

	async fn get_shard_status(&self, shard_id: ShardId) -> Result<ShardStatus> {
		let shards = self.shards.lock().unwrap();
		let Some(shard) = shards.get(&shard_id) else {
			return Ok(ShardStatus::Offline);
		};
		if shard.online == shard.members.len() {
			return Ok(ShardStatus::Online);
		}
		if shard.commitments.len() == shard.members.len() {
			return Ok(ShardStatus::Committed);
		}
		Ok(ShardStatus::Created)
	}

	async fn get_shard_commitment(&self, shard_id: ShardId) -> Result<Option<Commitment>> {
		let shards = self.shards.lock().unwrap();
		let Some(shard) = shards.get(&shard_id) else {
			return Ok(None);
		};
		if shard.commitments.len() != shard.members.len() {
			return Ok(None);
		}
		let commitments: Vec<_> = shard
			.commitments
			.iter()
			.map(|commitment| {
				VerifiableSecretSharingCommitment::deserialize(commitment.clone()).unwrap()
			})
			.collect();
		let commitments = commitments.iter().collect::<Vec<_>>();
		Ok(Some(sum_commitments(&commitments).unwrap().serialize()))
	}

	async fn get_shard_tasks(&self, shard_id: ShardId) -> Result<Vec<TaskId>> {
		let assigned_tasks = self.assigned_tasks.lock().unwrap();
		let tasks = assigned_tasks
			.iter()
			.filter(|(_, shard)| **shard == shard_id)
			.map(|(task, _)| *task)
			.collect();
		Ok(tasks)
	}

	async fn get_task(&self, task_id: TaskId) -> Result<Option<Task>> {
		let tasks = self.tasks.lock().unwrap();
		Ok(tasks.get(&task_id).map(|task| task.task.clone()))
	}

	async fn get_task_submitter(&self, task_id: TaskId) -> Result<Option<PublicKey>> {
		let tasks = self.tasks.lock().unwrap();
		Ok(tasks.get(&task_id).unwrap().submitter.clone())
	}

	async fn get_batch_message(&self, batch: BatchId) -> Result<Option<GatewayMessage>> {
		let batches = self.batches.lock().unwrap();
		Ok(batches.get(&batch).map(|b| b.message.clone()))
	}

	async fn get_gateway(&self, _network: NetworkId) -> Result<Option<Gateway>> {
		Ok(Some([0; 32]))
	}

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		_stake_amount: u128,
	) -> Result<()> {
		let mut members = self.members.lock().unwrap();
		members.entry(network).or_default().push((self.public_key().clone(), peer_id));
		Ok(())
	}

	async fn submit_unregister_member(&self) -> Result<()> {
		Ok(())
	}

	async fn submit_heartbeat(&self) -> Result<()> {
		Ok(())
	}

	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		_proof_of_knowledge: ProofOfKnowledge,
	) -> Result<()> {
		let mut shards = self.shards.lock().unwrap();
		shards.get_mut(&shard_id).unwrap().commitments.push(commitment);
		Ok(())
	}

	async fn submit_online(&self, shard_id: ShardId) -> Result<()> {
		let mut shards = self.shards.lock().unwrap();
		shards.get_mut(&shard_id).unwrap().online += 1;
		Ok(())
	}

	async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		tasks.get_mut(&task_id).unwrap().result = Some(result);
		Ok(())
	}
}
