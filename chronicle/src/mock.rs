use crate::tasks::TaskSpawner;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::stream::BoxStream;
use futures::{stream, FutureExt, SinkExt, Stream, StreamExt};
use schnorr_evm::k256::ProjectivePoint;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use time_primitives::sp_runtime::traits::IdentifyAccount;
use time_primitives::{
	sp_core, AccountId, Balance, BlockHash, BlockNumber, ChainName, ChainNetwork, Commitment,
	Function, MemberStatus, NetworkId, Payload, PeerId, ProofOfKnowledge, PublicKey, Runtime,
	ShardId, ShardStatus, TaskDescriptor, TaskExecution, TaskId, TaskPhase, TaskResult, TssHash,
	TssSignature, TssSigningRequest,
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
	pub descriptor: TaskDescriptor,
	pub phase: TaskPhase,
	pub signature: Option<TssSignature>,
	pub signer: Option<PublicKey>,
	pub hash: Option<[u8; 32]>,
	pub result: Option<TaskResult>,
}

impl MockTask {
	pub fn new(descriptor: TaskDescriptor) -> Self {
		let phase = descriptor.function.initial_phase();
		Self {
			descriptor,
			phase,
			signature: None,
			signer: None,
			hash: None,
			result: None,
		}
	}

	fn execution(&self, task_id: TaskId) -> TaskExecution {
		TaskExecution::new(task_id, self.phase)
	}
}

type Networks = Arc<Mutex<HashMap<NetworkId, MockNetwork>>>;
type Members = Arc<Mutex<HashMap<NetworkId, Vec<(PublicKey, PeerId)>>>>;
type Shards = Arc<Mutex<HashMap<ShardId, MockShard>>>;
type Tasks = Arc<Mutex<HashMap<TaskId, MockTask>>>;
type AssignedTasks = Arc<Mutex<HashMap<TaskId, ShardId>>>;

#[derive(Clone, Default)]
pub struct Mock {
	public_key: Option<PublicKey>,
	account_id: Option<AccountId>,
	networks: Networks,
	members: Members,
	shards: Shards,
	tasks: Tasks,
	assigned_tasks: AssignedTasks,
	tss: Option<mpsc::Sender<TssSigningRequest>>,
}

impl Mock {
	pub fn instance(&self, id: u8) -> Self {
		let mut mock = self.clone();
		let public_key = PublicKey::Sr25519(sp_core::sr25519::Public::from_raw([id; 32]));
		mock.public_key = Some(public_key.clone());
		mock.account_id = Some(public_key.into_account());
		mock
	}

	pub fn with_tss(&mut self, tss: mpsc::Sender<TssSigningRequest>) {
		self.tss = Some(tss);
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

	async fn submit_task_signature_core(
		self,
		task_id: TaskId,
		signature: TssSignature,
	) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		let task = tasks.get_mut(&task_id).unwrap();
		task.signature = Some(signature);
		task.phase = TaskPhase::Write;
		task.signer = Some(self.public_key().clone());
		Ok(())
	}

	async fn submit_task_result_core(self, task_id: TaskId, result: TaskResult) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		let task = tasks.get_mut(&task_id).unwrap();
		task.result = Some(result);
		Ok(())
	}

	async fn submit_task_hash_core(self, task_id: TaskId, hash: [u8; 32]) -> Result<()> {
		let mut tasks = self.tasks.lock().unwrap();
		let task = tasks.get_mut(&task_id).unwrap();
		task.phase = TaskPhase::Read;
		task.hash = Some(hash);
		Ok(())
	}

	async fn tss_sign(
		&self,
		block_number: BlockNumber,
		shard_id: ShardId,
		task_id: TaskId,
		payload: &[u8],
	) -> Result<(TssHash, TssSignature)> {
		if let Some(mut tss) = self.tss.clone() {
			let (tx, rx) = oneshot::channel();
			tss.send(TssSigningRequest {
				request_id: task_id,
				shard_id,
				block_number,
				data: payload.to_vec(),
				tx,
			})
			.await?;
			Ok(rx.await?)
		} else {
			Ok(([0; 32], [0; 64]))
		}
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

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
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

	async fn get_shards(&self, _block: BlockHash, account: &AccountId) -> Result<Vec<ShardId>> {
		let shards = self.shards.lock().unwrap();
		let shards = shards
			.iter()
			.filter(|(_, shard)| shard.members.iter().any(|(acc, _)| acc == account))
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

	async fn get_shard_status(&self, _block: BlockHash, shard_id: ShardId) -> Result<ShardStatus> {
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

	async fn get_task_signer(&self, task_id: TaskId) -> Result<Option<PublicKey>> {
		let tasks = self.tasks.lock().unwrap();
		Ok(tasks.get(&task_id).unwrap().signer.clone())
	}

	async fn get_task_hash(&self, task_id: TaskId) -> Result<Option<[u8; 32]>> {
		let tasks = self.tasks.lock().unwrap();
		Ok(tasks.get(&task_id).unwrap().hash)
	}

	async fn get_gateway(&self, _network: NetworkId) -> Result<Option<[u8; 20]>> {
		Ok(Some([0; 20]))
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

	async fn submit_heartbeat(&self, _: u64) -> Result<()> {
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

	async fn submit_task_hash(&self, task_id: TaskId, hash: [u8; 32]) -> Result<()> {
		self.clone().submit_task_hash(task_id, hash).await.unwrap();
		Ok(())
	}

	async fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Result<()> {
		self.clone().submit_task_signature_core(task_id, signature).await.unwrap();
		Ok(())
	}

	async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
		self.clone().submit_task_result_core(task_id, result).await.unwrap();
		Ok(())
	}
}

impl TaskSpawner for Mock {
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		stream::iter(std::iter::successors(Some(0), |n| Some(n + 1)))
			.then(|e| async move {
				tokio::time::sleep(Duration::from_secs(1)).await;
				e
			})
			.boxed()
	}

	fn execute_read(
		&self,
		_target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		function: Function,
		block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let payload = serde_json::to_string(&function).unwrap();
		let spawner = self.clone();
		Box::pin(async move {
			let (hash, signature) =
				spawner.tss_sign(block_num, shard_id, task_id, payload.as_bytes()).await?;
			spawner
				.submit_task_result_core(
					task_id,
					TaskResult {
						shard_id,
						payload: Payload::Hashed(hash),
						signature,
					},
				)
				.await?;
			Ok(())
		})
	}

	fn execute_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		payload: Vec<u8>,
		block_num: u32,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let spawner = self.clone();
		Box::pin(async move {
			let (_hash, sig) = spawner.tss_sign(block_num, shard_id, task_id, &payload).await?;
			spawner.submit_task_signature_core(task_id, sig).await?;
			Ok(())
		})
	}

	fn execute_write(
		&self,
		task_id: TaskId,
		_: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().submit_task_hash_core(task_id, [0; 32]).boxed()
	}
}
