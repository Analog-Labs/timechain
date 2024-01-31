use crate::tasks::TaskSpawner;
use anyhow::Result;
use futures::stream::BoxStream;
use futures::{future, stream, FutureExt, Stream, StreamExt};
use schnorr_evm::k256::ProjectivePoint;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use time_primitives::{
	sp_core, AccountId, BlockHash, BlockNumber, ChainName, ChainNetwork, Commitment, Function,
	MemberStatus, NetworkId, PeerId, ProofOfKnowledge, PublicKey, Runtime, ShardId, ShardStatus,
	TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId, TaskPhase, TaskResult, TaskStatus,
	TssSignature,
};
use tss::{sum_commitments, VerifiableSecretSharingCommitment, VerifyingKey};

#[derive(Clone, Debug)]
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
	pub descriptor: TaskDescriptor,
	pub phase: TaskPhase,
	pub status: TaskStatus,
	pub signature: Option<TssSignature>,
	pub results: Vec<TaskResult>,
}

impl MockTask {
	pub fn new(descriptor: TaskDescriptor) -> Self {
		let phase = if descriptor.function.is_gmp() {
			TaskPhase::Sign
		} else if descriptor.function.is_payable() {
			//TaskPhase::Write(self.public_key().clone().into())
			todo!()
		} else {
			TaskPhase::Read(None)
		};
		Self {
			descriptor,
			phase,
			status: TaskStatus::Created,
			signature: None,
			results: vec![],
		}
	}

	fn execution(&self, task_id: TaskId) -> TaskExecution {
		TaskExecution::new(task_id, self.results.len() as _, 0, self.phase.clone())
	}
}

#[derive(Clone)]
pub struct Mock {
	public_key: PublicKey,
	account_id: AccountId,
	networks: Arc<Mutex<HashMap<NetworkId, MockNetwork>>>,
	shards: Arc<Mutex<HashMap<ShardId, MockShard>>>,
	tasks: Arc<Mutex<HashMap<TaskId, MockTask>>>,
	assigned_tasks: Arc<Mutex<HashMap<TaskId, ShardId>>>,
}

impl Mock {
	pub fn new() -> Self {
		let public_key = PublicKey::Sr25519(sp_core::sr25519::Public::from_raw([0; 32]));
		Self {
			public_key,
			account_id: [0; 32].into(),
			networks: Default::default(),
			shards: Default::default(),
			tasks: Default::default(),
			assigned_tasks: Default::default(),
		}
	}

	pub fn create_network(&self, chain_name: ChainName, chain_network: ChainNetwork) -> NetworkId {
		let mut networks = self.networks.lock().unwrap();
		let network_id = networks.len() as _;
		networks.insert(network_id, MockNetwork::new(chain_name, chain_network));
		network_id
	}

	pub fn create_shard(&self, members: Vec<AccountId>, threshold: u16) -> ShardId {
		let mut shards = self.shards.lock().unwrap();
		let shard_id = shards.len() as _;
		shards.insert(shard_id, MockShard::new(members, threshold));
		shard_id
	}

	pub fn create_online_shard(&self, members: Vec<AccountId>, threshold: u16) -> ShardId {
		let shard_id = self.create_shard(members, threshold);
		let mut shards = self.shards.lock().unwrap();
		let shard = shards.get_mut(&shard_id).unwrap();
		let public_key = VerifyingKey::new(ProjectivePoint::GENERATOR).to_bytes().unwrap();
		shard.commitments = vec![vec![public_key; threshold as _]; shard.members.len()];
		shard.online = shard.members.len();
		shard_id
	}

	pub fn create_task(&self, descriptor: TaskDescriptor) -> TaskId {
		let mut tasks = self.tasks.lock().unwrap();
		let task_id = tasks.len() as _;
		tasks.insert(task_id, MockTask::new(descriptor));
		task_id
	}

	pub fn assign_task(&self, task_id: TaskId, shard_id: ShardId) {
		let mut assigned_tasks = self.assigned_tasks.lock().unwrap();
		assigned_tasks.insert(task_id, shard_id);
	}

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
		&self.public_key
	}

	fn account_id(&self) -> &AccountId {
		&self.account_id
	}

	async fn get_block_time_in_ms(&self) -> Result<u64> {
		Ok(6000)
	}

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		stream::iter(std::iter::successors(Some(([0; 32].into(), 0)), |(_, n)| {
			let n = n + 1;
			Some(([n as _; 32].into(), n))
		}))
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

	async fn get_member_peer_id(
		&self,
		_block: BlockHash,
		account: &AccountId,
	) -> Result<Option<PeerId>> {
		Ok(Some((*account).clone().into()))
	}

	async fn get_heartbeat_timeout(&self) -> Result<u64> {
		Ok(1000)
	}

	async fn get_min_stake(&self) -> Result<u128> {
		Ok(0)
	}

	async fn get_shards(&self, _block: BlockHash, account: &AccountId) -> Result<Vec<ShardId>> {
		let shards = self.shards.lock().unwrap();
		let shards = shards
			.iter()
			.filter(|(_, shard)| shard.members.iter().find(|(acc, _)| acc == account).is_some())
			.map(|(shard, _)| *shard)
			.collect();
		Ok(shards)
	}

	async fn get_shard_members(
		&self,
		_block: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<(AccountId, MemberStatus)>> {
		let shards = self.shards.lock().unwrap();
		let members = shards.get(&shard_id).map(|shard| shard.members.clone()).unwrap_or_default();
		Ok(members)
	}

	async fn get_shard_threshold(&self, _block: BlockHash, shard_id: ShardId) -> Result<u16> {
		let shards = self.shards.lock().unwrap();
		let threshold = shards.get(&shard_id).map(|shard| shard.threshold).unwrap_or_default();
		Ok(threshold)
	}

	async fn get_shard_status(
		&self,
		_block: BlockHash,
		shard_id: ShardId,
	) -> Result<ShardStatus<BlockNumber>> {
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
		Ok(ShardStatus::Created(0))
	}

	async fn get_shard_commitment(
		&self,
		_block: BlockHash,
		shard_id: ShardId,
	) -> Result<Commitment> {
		let shards = self.shards.lock().unwrap();
		let commitments =
			shards.get(&shard_id).map(|shard| shard.commitments.clone()).unwrap_or_default();
		let commitments: Vec<_> = commitments
			.iter()
			.map(|commitment| {
				VerifiableSecretSharingCommitment::deserialize(commitment.clone()).unwrap()
			})
			.collect();
		let commitments = commitments.iter().collect::<Vec<_>>();
		Ok(sum_commitments(&commitments).unwrap().serialize())
	}

	async fn get_shard_tasks(
		&self,
		_block: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<TaskExecution>> {
		let assigned_tasks = self.assigned_tasks.lock().unwrap();
		let tasks = self.tasks.lock().unwrap();
		let tasks = assigned_tasks
			.iter()
			.filter(|(_, shard)| **shard == shard_id)
			.map(|(task, _)| tasks.get(task).unwrap().execution(*task))
			.collect();
		Ok(tasks)
	}

	async fn get_task(&self, _block: BlockHash, task_id: TaskId) -> Result<Option<TaskDescriptor>> {
		let tasks = self.tasks.lock().unwrap();
		Ok(tasks.get(&task_id).map(|task| task.descriptor.clone()))
	}

	async fn get_task_signature(&self, task_id: TaskId) -> Result<Option<TssSignature>> {
		let tasks = self.tasks.lock().unwrap();
		Ok(tasks.get(&task_id).unwrap().signature)
	}

	async fn get_gateway(&self, _network: NetworkId) -> Result<Option<Vec<u8>>> {
		Ok(Some([0; 20].into()))
	}

	async fn submit_register_member(
		&self,
		_network: NetworkId,
		_peer_id: PeerId,
		_stake_amount: u128,
	) -> Result<()> {
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

	async fn submit_task_hash(
		&self,
		task_id: TaskId,
		_cycle: TaskCycle,
		hash: Vec<u8>,
	) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		tasks.get_mut(&task_id).unwrap().phase = TaskPhase::Read(Some(hash.try_into().unwrap()));
		Ok(())
	}

	async fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		let task = tasks.get_mut(&task_id).unwrap();
		task.signature = Some(signature);
		task.phase = TaskPhase::Write(self.public_key().clone());
		Ok(())
	}

	async fn submit_task_result(
		&self,
		task_id: TaskId,
		_cycle: TaskCycle,
		result: TaskResult,
	) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		let task = tasks.get_mut(&task_id).unwrap();
		task.results.push(result);
		task.status = TaskStatus::Completed;
		Ok(())
	}

	async fn submit_task_error(
		&self,
		task_id: TaskId,
		_cycle: TaskCycle,
		error: TaskError,
	) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		tasks.get_mut(&task_id).unwrap().status = TaskStatus::Failed { error };
		Ok(())
	}
}

impl TaskSpawner for Mock {
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		stream::iter(std::iter::successors(Some(0), |n| Some(n + 1))).boxed()
	}

	fn chain_id(&self) -> u64 {
		0
	}

	fn execute_read(
		&self,
		_target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		_function: Function,
		_hash: Option<[u8; 32]>,
		_block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.submit_task_result(
			task_id,
			cycle,
			TaskResult {
				shard_id,
				hash: [0; 32],
				signature: [0; 64],
			},
		)
		.boxed()
	}

	fn execute_sign(
		&self,
		_: ShardId,
		task_id: TaskId,
		_: TaskCycle,
		_: Vec<u8>,
		_: u32,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.submit_task_signature(task_id, [0; 64]).boxed()
	}

	fn execute_write(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		_: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.submit_task_hash(task_id, cycle, [0; 32].into()).boxed()
	}
}
