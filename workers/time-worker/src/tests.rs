use crate::TimeWorkerParams;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use sc_client_api::Backend;
use sc_consensus::{BoxJustificationImport, ForkChoiceStrategy};
use sc_network::NetworkSigner;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient, TestNet,
	TestNetFactory,
};
use sc_transaction_pool::BasicPool;
use sc_transaction_pool_api::LocalTransactionPool;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sc_transaction_pool_api::TransactionPool;
use sp_api::BlockT;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus::BlockOrigin;
use sp_core::offchain::testing::{TestOffchainExt, TestTransactionPoolExt};
use sp_core::offchain::TransactionPoolExt;
use sp_keystore::testing::MemoryKeystore;
use sp_keystore::Keystore;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::Block as sp_block;
use sp_runtime::traits::Extrinsic;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use time_primitives::{
	AccountId, MembersApi, Network, PeerId, PublicKey, ShardId, ShardsApi, TssId, TssPublicKey,
	TssRequest, TssSignature, TIME_KEY_TYPE,
};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

#[derive(Default)]
struct InnerMockApi {
	shard_counter: ShardId,
	shards: HashMap<AccountId, Vec<ShardId>>,
	members: HashMap<ShardId, Vec<AccountId>>,
	pubkey: Option<TssPublicKey>,
}

impl InnerMockApi {
	fn create_shard(&mut self, members: Vec<AccountId>) -> ShardId {
		let id = self.shard_counter;
		self.shard_counter += 1;
		for member in members.clone() {
			self.shards.entry(member).or_default().push(id);
		}
		self.members.insert(id, members);
		id
	}

	fn get_shards(&self, peer_id: AccountId) -> Vec<ShardId> {
		self.shards.get(&peer_id).cloned().unwrap_or_default()
	}

	fn get_shard_members(&self, shard_id: ShardId) -> Vec<AccountId> {
		self.members.get(&shard_id).cloned().unwrap_or_default()
	}

	fn get_shard_threshold(&self, _shard_id: ShardId) -> u16 {
		3
	}

	fn submit_tss_public_key(&mut self, shard_id: ShardId, public_key: TssPublicKey) {
		self.pubkey = Some(public_key);
	}
}

#[derive(Clone, Default)]
struct MockApi {
	inner: Arc<Mutex<InnerMockApi>>,
}

impl MockApi {
	pub fn create_shard(&self, members: Vec<AccountId>) -> ShardId {
		self.inner.lock().unwrap().create_shard(members)
	}
	pub fn get_tss_key(&self) -> Option<TssPublicKey> {
		self.inner.lock().unwrap().pubkey
	}
}

sp_api::mock_impl_runtime_apis! {
	impl ShardsApi<Block> for MockApi {

		fn get_shards(account: &AccountId) -> Vec<ShardId> {
			self.inner.lock().unwrap().get_shards((*account).clone().into())
		}

		fn get_shard_members(shard_id: ShardId) -> Vec<AccountId> {
			self.inner.lock().unwrap().get_shard_members(shard_id)
		}

		fn get_shard_threshold(shard_id: ShardId) -> u16 {
			self.inner.lock().unwrap().get_shard_threshold(shard_id)
		}

		fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey) {
			self.inner.lock().unwrap().submit_tss_public_key(shard_id, public_key);
		}
	}

	impl MembersApi<Block> for MockApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>{
			Some((*account).clone().into())
		}
		fn submit_register_member(network: Network, public_key: PublicKey, peer_id: PeerId) {}
		fn submit_heartbeat(public_key: PublicKey) {}
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
	let public_key = frost_evm::VerifyingKey::from_bytes(public_key)?;
	let signature = frost_evm::Signature::from_bytes(signature)?;
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

	async fn start_tasks(
		&self,
		_block_hash: <B as sp_block>::Hash,
		_block_num: u64,
		_shard_id: ShardId,
	) -> Result<()> {
		Ok(())
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
#[ignore]
async fn tss_smoke() -> Result<()> {
	env_logger::try_init().ok();

	let client = Arc::new(substrate_test_runtime_client::new());
	let spawner = sp_core::testing::TaskExecutor::new();
	let txpool =
		BasicPool::new_full(Default::default(), true.into(), None, spawner.clone(), client.clone());
	let task_executor = MockTaskExecutor::<Block>::new();
	let mut net = MockNetwork::default();
	let api = Arc::new(MockApi::default());
	let mut peers = vec![];
	let mut tss = vec![];
	for i in 0..3 {
		let keystore = MemoryKeystore::new();
		let collector = keystore.sr25519_generate_new(TIME_KEY_TYPE, Some("//Alice")).unwrap();
		let factory = OffchainTransactionPoolFactory::new(txpool.clone());

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
		peers.push(peer_id);
		tss.push(tss_tx);
		tokio::task::spawn(crate::start_timeworker_gadget(TimeWorkerParams {
			_block: PhantomData,
			client: net.peer(i).client().as_client(),
			runtime: api.clone(),
			kv: keystore.into(),
			network: net.peer(i).network_service().clone(),
			offchain_tx_pool_factory: factory,
			peer_id: peers[i],
			tss_request: tss_rx,
			protocol_request: protocol_rx,
			task_executor: task_executor.clone(),
			public_key: pubkey_from_bytes([i as u8; 32]),
		}));
	}
	net.run_until_connected().await;

	let client: Vec<_> = (0..3).map(|i| net.peer(i).client().as_client()).collect();

	let peers_account_id: Vec<AccountId> = peers.iter().map(|p| (*p).into()).collect();
	log::info!("creating shard with members {:#?}", peers_account_id);
	api.create_shard(peers_account_id.into());

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

	// TODO: after collector completes dkg all other nodes have time until a task
	// is scheduled to complete. we should require all members to submit the public
	// key before scheduling.
	sp_std::if_std! {
		println!("{:?}", txpool.status());
	}

	let mut tss_public_key = None;
	loop {
		if let Some(pubkey) = api.get_tss_key() {
			tss_public_key = Some(pubkey);
		}
	}
	let public_key = tss_public_key.unwrap();
	log::info!("dkg returned a public key");

	let block_number = client[0].chain_info().finalized_number;
	let message = [1u8; 32];
	let mut rxs = vec![];
	for tss in &mut tss {
		let (tx, rx) = oneshot::channel();
		tss.send(TssRequest {
			request_id: TssId(1, 1),
			shard_id: 0,
			block_number,
			data: message.to_vec(),
			tx,
		})
		.await?;
		rxs.push(rx);
	}
	for rx in rxs {
		let signature = rx.await?;
		verify_tss_signature(public_key, &message, signature)?;
	}

	Ok(())
}
