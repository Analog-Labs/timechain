use crate::network::{TimeWorker, TimeWorkerParams};
use crate::tx_submitter::TransactionSubmitter;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use sc_consensus::{BoxJustificationImport, ForkChoiceStrategy};
use sc_network::NetworkSigner;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient, TestNet,
	TestNetFactory,
};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sc_transaction_pool_api::RejectAllTxPool;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus::BlockOrigin;
use sp_keystore::testing::MemoryKeystore;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as sp_block, IdentifyAccount};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use time_primitives::{
	AccountId, BlockTimeApi, Commitment, MembersApi, Network, PeerId, ProofOfKnowledge, PublicKey,
	ShardId, ShardStatus, ShardsApi, TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId,
	TaskResult, TasksApi, TssId, TssPublicKey, TssSignature, TssSigningRequest, TxResult,
};
use tracing::{span, Level};
use tss::{compute_group_commitment, VerifiableSecretSharingCommitment};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

#[derive(Default)]
struct InnerMockApi {
	shard_counter: ShardId,
	shards: HashMap<AccountId, Vec<ShardId>>,
	members: HashMap<ShardId, Vec<AccountId>>,
	thresholds: HashMap<ShardId, u16>,
	commitments: HashMap<ShardId, Vec<Commitment>>,
	online: HashMap<ShardId, usize>,
}

impl InnerMockApi {
	fn create_shard(&mut self, members: Vec<AccountId>, threshold: u16) -> ShardId {
		let id = self.shard_counter;
		self.shard_counter += 1;
		for member in members.clone() {
			self.shards.entry(member).or_default().push(id);
		}
		self.members.insert(id, members);
		self.thresholds.insert(id, threshold);
		id
	}

	fn get_shards(&self, peer_id: AccountId) -> Vec<ShardId> {
		self.shards.get(&peer_id).cloned().unwrap_or_default()
	}

	fn get_shard_members(&self, shard_id: ShardId) -> Vec<AccountId> {
		self.members.get(&shard_id).cloned().unwrap_or_default()
	}

	fn get_shard_threshold(&self, shard_id: ShardId) -> u16 {
		self.thresholds.get(&shard_id).copied().unwrap_or_default()
	}

	fn get_shard_status(&self, shard_id: ShardId) -> ShardStatus<u32> {
		let Some(members) = self.members.get(&shard_id) else {
			return ShardStatus::Offline;
		};
		let num_members = members.len();
		if self.online.get(&shard_id).copied().unwrap_or_default() == num_members {
			return ShardStatus::Online;
		}
		if self.commitments.get(&shard_id).map(|cs| cs.len()).unwrap_or_default() == num_members {
			return ShardStatus::Committed;
		}
		ShardStatus::Created(0)
	}

	fn get_shard_commitment(&self, shard_id: ShardId) -> Commitment {
		let Some(commitments) = self.commitments.get(&shard_id) else {
			return Default::default();
		};
		let commitments: Vec<_> = commitments
			.iter()
			.map(|commitment| {
				VerifiableSecretSharingCommitment::deserialize(commitment.clone()).unwrap()
			})
			.collect();
		compute_group_commitment(&commitments).serialize()
	}

	fn submit_commitment(
		&mut self,
		shard_id: ShardId,
		_member: PublicKey,
		commitment: Commitment,
		_proof_of_knowledge: ProofOfKnowledge,
	) {
		self.commitments.entry(shard_id).or_default().push(commitment);
	}

	fn submit_online(&mut self, shard_id: ShardId, _member: PublicKey) {
		*self.online.entry(shard_id).or_default() += 1;
	}
}

#[derive(Clone, Default)]
struct MockApi {
	inner: Arc<Mutex<InnerMockApi>>,
}

impl MockApi {
	pub fn create_shard(&self, members: Vec<AccountId>, threshold: u16) -> ShardId {
		self.inner.lock().unwrap().create_shard(members, threshold)
	}

	pub fn shard_status(&self, shard_id: ShardId) -> ShardStatus<u32> {
		self.inner.lock().unwrap().get_shard_status(shard_id)
	}

	pub fn shard_public_key(&self, shard_id: ShardId) -> TssPublicKey {
		self.inner.lock().unwrap().get_shard_commitment(shard_id)[0]
	}
}

sp_api::mock_impl_runtime_apis! {
	impl ShardsApi<Block> for MockApi {
		fn get_shards(&self, account: &AccountId) -> Vec<ShardId> {
			self.inner.lock().unwrap().get_shards((*account).clone())
		}

		fn get_shard_members(&self, shard_id: ShardId) -> Vec<AccountId> {
			self.inner.lock().unwrap().get_shard_members(shard_id)
		}

		fn get_shard_threshold(&self, shard_id: ShardId) -> u16 {
			self.inner.lock().unwrap().get_shard_threshold(shard_id)
		}

		fn get_shard_status(&self, shard_id: ShardId) -> ShardStatus<u32> {
			self.inner.lock().unwrap().get_shard_status(shard_id)
		}

		fn get_shard_commitment(&self, shard_id: ShardId) -> Commitment {
			self.inner.lock().unwrap().get_shard_commitment(shard_id)
		}

		fn submit_commitment(&self, shard_id: ShardId, member: PublicKey, commitment: Commitment, proof_of_knowledge: ProofOfKnowledge) -> TxResult {
			self.inner.lock().unwrap().submit_commitment(shard_id, member, commitment, proof_of_knowledge);
			Ok(())
		}

		fn submit_online(&self, shard_id: ShardId, member: PublicKey) -> TxResult {
			self.inner.lock().unwrap().submit_online(shard_id, member);
			Ok(())
		}
	}

	impl MembersApi<Block> for MockApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>{
			Some((*account).clone().into())
		}
		fn get_heartbeat_timeout() -> u64 {
			1000
		}
		fn submit_register_member(_network: Network, _public_key: PublicKey, _peer_id: PeerId) -> TxResult { Ok(()) }
		fn submit_heartbeat(_public_key: PublicKey) -> TxResult { Ok(()) }
	}

	impl TasksApi<Block> for MockApi{
		fn get_shard_tasks(_shard_id: ShardId) -> Vec<TaskExecution> { vec![] }
		fn get_task(_task_id: TaskId) -> Option<TaskDescriptor> { None }
		fn submit_task_hash(_shard_id: ShardId, _task_id: TaskId, _hash: Vec<u8>) -> TxResult { Ok(()) }
		fn submit_task_result(_task_id: TaskId, _cycle: TaskCycle, _status: TaskResult) -> TxResult { Ok(()) }
		fn submit_task_error(_task_id: TaskId, _cycle: TaskCycle, _error: TaskError) -> TxResult { Ok(()) }
	}

	impl BlockTimeApi<Block> for MockApi{
		fn get_block_time_in_msec() -> u64{
			1000000000000
		}
	}

}

impl ProvideRuntimeApi<Block> for MockApi {
	type Api = Self;

	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.clone().into()
	}
}

fn verify_tss_signature(
	public_key: TssPublicKey,
	message: &[u8],
	signature: TssSignature,
) -> Result<()> {
	let public_key = schnorr_evm::VerifyingKey::from_bytes(public_key)?;
	let signature = schnorr_evm::Signature::from_bytes(signature)?;
	public_key.verify(message, &signature)?;
	Ok(())
}

#[derive(Clone)]
struct MockTaskExecutor<B> {
	_block: PhantomData<B>,
}

impl<B: sp_block> MockTaskExecutor<B> {
	fn new() -> Self {
		Self { _block: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B> time_primitives::TaskExecutor<B> for MockTaskExecutor<B>
where
	B: sp_block,
{
	fn network(&self) -> Network {
		Network::Ethereum
	}

	async fn poll_block_height(&mut self) {

	}

	fn process_tasks(
		&mut self,
		_block_hash: <B as sp_block>::Hash,
		_block_num: u64,
		_shard_id: ShardId,
	) -> Result<Vec<TssId>> {
		Ok(vec![])
	}
}

#[derive(Default)]
struct MockNetwork {
	inner: TestNet,
}

impl TestNetFactory for MockNetwork {
	type Verifier = PassThroughVerifier;
	type PeerData = ();
	type BlockImport = PeersClient;

	fn make_verifier(&self, _client: PeersClient, _peer_data: &()) -> Self::Verifier {
		PassThroughVerifier::new(true)
	}

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<BoxJustificationImport<Block>>,
		Self::PeerData,
	) {
		self.inner.make_block_import(client)
	}

	fn peer(&mut self, i: usize) -> &mut Peer<(), Self::BlockImport> {
		self.inner.peer(i)
	}

	fn peers(&self) -> &Vec<Peer<(), Self::BlockImport>> {
		self.inner.peers()
	}

	fn peers_mut(&mut self) -> &mut Vec<Peer<(), Self::BlockImport>> {
		self.inner.peers_mut()
	}

	fn mut_peers<F: FnOnce(&mut Vec<Peer<(), Self::BlockImport>>)>(&mut self, closure: F) {
		self.inner.mut_peers(closure)
	}
}

impl MockNetwork {
	async fn poll(&mut self) {
		futures::future::poll_fn::<(), _>(|cx| {
			self.inner.poll(cx);
			Poll::Pending
		})
		.await;
	}
}

#[tokio::test]
async fn tss_smoke() -> Result<()> {
	env_logger::try_init().ok();
	log_panics::init();

	let mut net = MockNetwork::default();
	let api = Arc::new(MockApi::default());

	let task_executor = MockTaskExecutor::<Block>::new();

	let mut peers = vec![];
	let mut pub_keys = vec![];
	let mut tss = vec![];
	for i in 0..3 {
		let (protocol_tx, protocol_rx) = async_channel::unbounded();
		let (tss_tx, tss_rx) = mpsc::channel(10);
		net.add_full_peer_with_config(FullPeerConfig {
			request_response_protocols: vec![crate::protocol_config(protocol_tx)],
			..Default::default()
		});
		let peer_id = net
			.peer(i)
			.network_service()
			.sign_with_local_identity([])?
			.public_key
			.try_into_ed25519()?
			.to_bytes();

		pub_keys.push(pubkey_from_bytes(peer_id));
		peers.push(peer_id);
		tss.push(tss_tx);

		let tx_submitter = TransactionSubmitter::new(
			false,
			MemoryKeystore::new().into(),
			OffchainTransactionPoolFactory::new(RejectAllTxPool::default()),
			net.peer(i).client().as_client(),
			api.clone(),
		);

		let worker = TimeWorker::new(TimeWorkerParams {
			_block: PhantomData,
			client: net.peer(i).client().as_client(),
			runtime: api.clone(),
			network: net.peer(i).network_service().clone(),
			peer_id: peers[i],
			tss_request: tss_rx,
			protocol_request: protocol_rx,
			task_executor: task_executor.clone(),
			tx_submitter: tx_submitter.clone(),
			public_key: pub_keys[i].clone(),
		});
		tokio::task::spawn(async move {
			let span = span!(Level::INFO, "span");
			worker.run(&span).await
		});
	}

	tracing::info!("waiting for peers to connect");
	net.run_until_connected().await;

	let client: Vec<_> = (0..3).map(|i| net.peer(i).client().as_client()).collect();
	let peers_account_id: Vec<AccountId> =
		pub_keys.iter().map(|p| (*p).clone().into_account()).collect();
	let shard_id = api.create_shard(peers_account_id, 2);

	tokio::task::spawn(async move {
		let mut block_timer = tokio::time::interval(Duration::from_secs(1));
		loop {
			futures::select! {
				_ = block_timer.tick().fuse() => {
					let peer = net.peer(0);
					let best_hash = peer.client().info().best_hash;
					peer.generate_blocks_at(
						BlockId::Hash(best_hash),
						1,
						BlockOrigin::ConsensusBroadcast,
						|builder| builder.build().unwrap().block,
						false,
						true,
						false,
						ForkChoiceStrategy::LongestChain,
					);
				}
				_ = net.poll().fuse() => {}
			}
		}
	});

	tracing::info!("waiting for shard to go online");
	while api.shard_status(shard_id) != ShardStatus::Online {
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	}
	let public_key = api.shard_public_key(shard_id);

	let block_number = client[0].chain_info().finalized_number;
	let message = [1u8; 32];
	let mut rxs = FuturesUnordered::new();
	for tss in &mut tss {
		let (tx, rx) = oneshot::channel();
		tss.send(TssSigningRequest {
			request_id: TssId(1, 1),
			shard_id: 0,
			block_number,
			data: message.to_vec(),
			tx,
		})
		.await?;
		rxs.push(rx);
	}
	let signature = rxs.next().await.unwrap()?;
	verify_tss_signature(public_key, &message, signature.1)?;

	Ok(())
}
