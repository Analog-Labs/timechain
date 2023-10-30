use super::service::{TimeWorker, TimeWorkerParams};
use crate::substrate::Substrate;
use crate::tasks::TaskExecutor;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use futures::stream;
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
use sp_runtime::generic::BlockId;
use sp_runtime::traits::IdentifyAccount;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use time_primitives::{
	AccountId, BlockHash, BlockNumber, BlockTimeApi, Commitment, MembersApi, MembersPayload,
	Network, PeerId, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, ShardsApi, ShardsPayload,
	TaskCycle, TaskDescriptor, TaskError, TaskExecution, TaskId, TaskResult, TasksApi,
	TasksPayload, TssId, TssPublicKey, TssSignature, TssSigningRequest, TxResult,
};
use tracing::{span, Level};
use tss::{sum_commitments, VerifiableSecretSharingCommitment};

use tc_subxt::AccountInterface;
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
		tracing::info!("shard created");
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
		let commitments = commitments.iter().collect::<Vec<_>>();
		sum_commitments(&commitments).unwrap().serialize()
	}

	fn submit_commitment(
		&mut self,
		shard_id: ShardId,
		commitment: Commitment,
		_proof_of_knowledge: ProofOfKnowledge,
	) {
		self.commitments.entry(shard_id).or_default().push(commitment);
	}

	fn submit_online(&mut self, shard_id: ShardId) {
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
	}

	impl MembersApi<Block> for MockApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>{
			Some((*account).clone().into())
		}
		fn get_heartbeat_timeout() -> u64 {
			1000
		}
	}

	impl TasksApi<Block> for MockApi{
		fn get_shard_tasks(_shard_id: ShardId) -> Vec<TaskExecution> { vec![] }
		fn get_task(_task_id: TaskId) -> Option<TaskDescriptor> { None }
	}

	impl BlockTimeApi<Block> for MockApi{
		fn get_block_time_in_msec() -> u64{
			6000
		}
	}

	impl time_primitives::SubmitTransactionApi<Block> for MockApi {
		fn submit_transaction(_: Vec<u8>) -> TxResult {
			Ok(())
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
struct MockTaskExecutor {}

impl TaskExecutor for MockTaskExecutor {
	fn network(&self) -> Network {
		Network::Ethereum
	}

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		Box::pin(stream::iter(vec![1]))
	}

	fn process_tasks(
		&mut self,
		_block_hash: BlockHash,
		_block_number: BlockNumber,
		_shard_id: ShardId,
		_target_block_height: u64,
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

#[derive(Clone)]
struct MockSubxt {
	public_key: [u8; 32],
	api: Arc<MockApi>,
}

impl TasksPayload for MockSubxt {
	fn submit_task_hash(&self, _: TaskId, _: TaskCycle, _: Vec<u8>) -> Vec<u8> {
		vec![]
	}

	fn submit_task_signature(&self, _: TaskId, _: TssSignature) -> Vec<u8> {
		vec![]
	}

	fn submit_task_result(&self, _: TaskId, _: TaskCycle, _: TaskResult) -> Vec<u8> {
		vec![]
	}

	fn submit_task_error(&self, _: TaskId, _: TaskCycle, _: TaskError) -> Vec<u8> {
		vec![]
	}
}

impl ShardsPayload for MockSubxt {
	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> Vec<u8> {
		self.api
			.inner
			.lock()
			.unwrap()
			.submit_commitment(shard_id, commitment, proof_of_knowledge);
		vec![]
	}

	fn submit_online(&self, shard_id: ShardId) -> Vec<u8> {
		self.api.inner.lock().unwrap().submit_online(shard_id);
		vec![]
	}
}

impl MembersPayload for MockSubxt {
	fn submit_register_member(&self, _: Network, _: PublicKey, _: PeerId) -> Vec<u8> {
		vec![]
	}

	fn submit_heartbeat(&self) -> Vec<u8> {
		vec![]
	}
}

impl AccountInterface for MockSubxt {
	fn increment_nonce(&self) {}

	fn public_key(&self) -> PublicKey {
		PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(self.public_key))
	}

	fn account_id(&self) -> AccountId {
		self.public_key.into()
	}

	fn nonce(&self) -> u64 {
		0
	}
}

#[tokio::test]
async fn tss_smoke() -> Result<()> {
	env_logger::try_init().ok();
	log_panics::init();

	let mut net = MockNetwork::default();
	let api = Arc::new(MockApi::default());

	let task_executor = MockTaskExecutor {};

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

		let substrate = Substrate::new(
			false,
			OffchainTransactionPoolFactory::new(RejectAllTxPool::default()),
			net.peer(i).client().as_client(),
			api.clone(),
			MockSubxt {
				public_key: peer_id,
				api: api.clone(),
			},
		);

		let n = net.peer(i).network_service().clone();
		let (n, net_request) =
			crate::network::create_network(Some((n, protocol_rx)), Default::default()).await?;

		let worker = TimeWorker::new(TimeWorkerParams {
			network: n,
			tss_request: tss_rx,
			net_request,
			task_executor: task_executor.clone(),
			substrate: substrate.clone(),
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
			block_number: block_number.try_into().unwrap(),
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
