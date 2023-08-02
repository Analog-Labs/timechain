use crate::{
	communication::time_protocol_name::gossip_protocol_name, start_timeworker_gadget,
	TimeWorkerParams,
};
use futures::{
	channel::mpsc::Receiver, future, stream::FuturesUnordered, Future, FutureExt, StreamExt,
};
use parking_lot::{Mutex, RwLock};
use sc_consensus::BoxJustificationImport;
use sc_consensus_grandpa::{
	block_import, run_grandpa_voter, Config, GenesisAuthoritySetProvider, GrandpaParams, LinkHalf,
	SharedVoterState,
};
use sc_keystore::LocalKeystore;
use sc_network::config::Role;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, Hash, PassThroughVerifier, Peer, PeersClient,
	PeersFullClient, TestNetFactory,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_consensus_grandpa::{
	AuthorityList, EquivocationProof, GrandpaApi, OpaqueKeyOwnershipProof, SetId,
};
use sp_core::{crypto::key_types::GRANDPA, sr25519, Pair};
use sp_runtime::traits::Header as HeaderT;
use std::{collections::HashMap, marker::PhantomData, sync::Arc, task::Poll, time::Duration};
use substrate_test_runtime_client::{
	runtime::{AccountId, BlockNumber},
	Ed25519Keyring, Keystore, LongestChain,
};
use time_primitives::{
	abstraction::TimeTssKey, sharding::Shard, SignatureData, TimeApi, TIME_KEY_TYPE as TimeKeyType,
};

const GRANDPA_PROTOCOL_NAME: &str = "/grandpa/1";
const TEST_GOSSIP_DURATION: Duration = Duration::from_millis(500);

type TestLinkHalf =
	LinkHalf<Block, PeersFullClient, LongestChain<substrate_test_runtime_client::Backend, Block>>;
type GrandpaPeerData = Mutex<Option<TestLinkHalf>>;
type GrandpaBlockImport = sc_consensus_grandpa::GrandpaBlockImport<
	substrate_test_runtime_client::Backend,
	Block,
	PeersFullClient,
	LongestChain<substrate_test_runtime_client::Backend, Block>,
>;
type GrandpaPeer = Peer<GrandpaPeerData, GrandpaBlockImport>;

// same as runtime
type GrandpaBlockNumber = u64;

/// Set of test accounts using [`time_primitives::crypto`] types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumIter)]
enum TimeKeyring {
	Alice,
	Bob,
	Charlie,
	Dave,
	Eve,
	Ferdie,
	One,
	Two,
}

impl TimeKeyring {
	/// Return key pair.
	pub fn pair(self) -> sr25519::Pair {
		sr25519::Pair::from_string(self.to_seed().as_str(), None).unwrap()
	}

	/// Return public key.
	pub fn public(self) -> sr25519::Public {
		self.pair().public()
	}

	/// Return seed string.
	pub fn to_seed(self) -> String {
		format!("//{self}")
	}
}

pub(crate) struct TimeTestNet {
	peers: Vec<GrandpaPeer>,
	test_net: TestApi,
}

impl TimeTestNet {
	pub(crate) fn new(n_authority: usize, n_full: usize, test_net: TestApi) -> Self {
		let capacity = n_authority + n_full;
		let mut net = TimeTestNet {
			peers: Vec::with_capacity(capacity),
			test_net,
		};
		for _ in 0..n_authority {
			net.add_authority_peer();
		}
		for _ in 0..n_full {
			net.add_full_peer();
		}
		net
	}
	pub(crate) fn add_authority_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![gossip_protocol_name(), GRANDPA_PROTOCOL_NAME.into()],
			is_authority: true,
			..Default::default()
		})
	}

	pub(crate) fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![gossip_protocol_name(), GRANDPA_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}
}

impl Default for TimeTestNet {
	fn default() -> Self {
		let genesis_authorities =
			vec![Ed25519Keyring::Alice, Ed25519Keyring::Bob, Ed25519Keyring::Charlie];
		Self::new(3, 0, TestApi::new(genesis_authorities))
	}
}

impl TestNetFactory for TimeTestNet {
	type Verifier = PassThroughVerifier;
	type BlockImport = GrandpaBlockImport;
	type PeerData = GrandpaPeerData;

	fn peers_mut(&mut self) -> &mut Vec<GrandpaPeer> {
		&mut self.peers
	}

	fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![gossip_protocol_name(), GRANDPA_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}

	fn make_verifier(&self, _client: PeersClient, _: &Self::PeerData) -> Self::Verifier {
		PassThroughVerifier::new(false) // use non-instant finality.
	}

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<BoxJustificationImport<Block>>,
		Self::PeerData,
	) {
		let (client, backend) = (client.as_client(), client.as_backend());
		let (import, link) = block_import(client, &self.test_net, LongestChain::new(backend), None)
			.expect("Could not create block import for fresh peer.");
		let justification_import = Box::new(import.clone());
		(BlockImportAdapter::new(import), Some(justification_import), Mutex::new(Some(link)))
	}

	fn peer(&mut self, i: usize) -> &mut GrandpaPeer {
		&mut self.peers[i]
	}

	fn peers(&self) -> &Vec<GrandpaPeer> {
		&self.peers
	}

	fn mut_peers<F: FnOnce(&mut Vec<GrandpaPeer>)>(&mut self, closure: F) {
		closure(&mut self.peers);
	}
}

#[derive(Default, Clone)]
pub(crate) struct TestApi {
	grandpa_peers: Vec<Ed25519Keyring>,
}

impl TestApi {
	fn new(grandpa_peers: Vec<Ed25519Keyring>) -> Self {
		TestApi { grandpa_peers }
	}

	fn authority_list(&self) -> AuthorityList {
		self.grandpa_peers
			.iter()
			.map(|key| (*key).public().into())
			.map(|id| (id, 1))
			.collect()
	}
}

impl GenesisAuthoritySetProvider<Block> for TestApi {
	fn get(&self) -> sp_blockchain::Result<AuthorityList> {
		Ok(self.authority_list())
	}
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = RuntimeApi;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		RuntimeApi {
			group_keys: Arc::new(Mutex::new(HashMap::new())),
			stored_signatures: Arc::new(Mutex::new(vec![])),
			test_api: self.clone(),
		}
		.into()
	}
}

// compiler gets confused and warns us about unused test_api
#[derive(Clone)]
pub(crate) struct RuntimeApi {
	pub stored_signatures: Arc<Mutex<Vec<SignatureData>>>,
	pub group_keys: Arc<Mutex<HashMap<u64, TimeTssKey>>>,
	test_api: TestApi,
}

sp_api::mock_impl_runtime_apis! {
	impl GrandpaApi<Block> for RuntimeApi {
		fn grandpa_authorities(&self) -> AuthorityList {
			self.test_api.authority_list()
		}

		fn current_set_id(&self) -> SetId {
			0
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: EquivocationProof<Hash, GrandpaBlockNumber>,
			_key_owner_proof: OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: SetId,
			_authority_id: sp_consensus_grandpa::AuthorityId,
		) -> Option<OpaqueKeyOwnershipProof> {
			None
		}
	}

	impl TimeApi<Block, AccountId, BlockNumber> for RuntimeApi {
		fn get_shards(&self) -> Vec<(u64, Shard)> {
			vec![(1, Shard::Three([
				TimeKeyring::Alice.public().into(),
				TimeKeyring::Bob.public().into(),
				TimeKeyring::Charlie.public().into(),
			]))]
		}

	}

}

// Spawns grandpa voters. Returns a future to spawn on the runtime.
fn initialize_grandpa(net: &mut TimeTestNet) -> impl Future<Output = ()> {
	let voters = FuturesUnordered::new();

	// initializing grandpa gadget per peer
	for (peer_id, key) in net.test_net.grandpa_peers.iter().enumerate() {
		let keystore = Arc::new(LocalKeystore::in_memory());
		keystore
			.ed25519_generate_new(GRANDPA, Some(&key.to_seed()))
			.expect("Creates authority key");

		let (net_service, link) = {
			// temporary needed for some reason
			let link = net.peers[peer_id]
				.data
				.lock()
				.take()
				.expect("link initialized at startup;config qed");
			(net.peers[peer_id].network_service().clone(), link)
		};

		let grandpa_params = GrandpaParams {
			config: Config {
				gossip_duration: TEST_GOSSIP_DURATION,
				justification_period: 32,
				keystore: Some(keystore),
				name: Some(format!("peer#{peer_id}")),
				local_role: Role::Authority,
				observer_enabled: true,
				telemetry: None,
				protocol_name: GRANDPA_PROTOCOL_NAME.into(),
			},
			link,
			network: net_service,
			voting_rule: (),
			prometheus_registry: None,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: None,
			sync: net.peers[peer_id].sync_service().clone(),
		};
		let voter =
			run_grandpa_voter(grandpa_params).expect("all in order with client and network");

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&voter);
		voters.push(voter);
	}

	voters.for_each(|_| async move {})
}

// Spawns time workers. Returns a future to spawn on the runtime.
fn initialize_time_worker<API>(
	net: &mut TimeTestNet,
	peers: Vec<(usize, &TimeKeyring, API, Receiver<(u64, u64, u64, [u8; 32])>)>,
) -> impl Future<Output = ()>
where
	API: ProvideRuntimeApi<Block> + Send + Sync + Default + 'static,
	API::Api: TimeApi<Block, AccountId, BlockNumber>,
{
	let time_workers = FuturesUnordered::new();

	// initializing time gadget per peer
	for (peer_id, key, api, sign_data_receiver) in peers.into_iter() {
		let peer = &net.peers[peer_id];

		let keystore = Arc::new(LocalKeystore::in_memory());
		keystore
			.sr25519_generate_new(TimeKeyType, Some(&key.to_seed()))
			.expect("Creates authority key");

		let time_params = TimeWorkerParams {
			client: peer.client().as_client(),
			backend: peer.client().as_backend(),
			runtime: api.into(),
			gossip_network: peer.network_service().clone(),
			kv: keystore,
			sign_data_receiver,
			sync_service: peer.sync_service().clone(),
			_block: PhantomData::default(),
			accountid: PhantomData::default(),
			_block_number: PhantomData::default(),
		};
		let gadget = start_timeworker_gadget::<_, _, _, _, _, _, _, _>(time_params);

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&gadget);
		time_workers.push(gadget);
	}

	time_workers.for_each(|_| async move {})
}

// run the voters to completion. provide a closure to be invoked after
// the voters are spawned but before blocking on them.
async fn run_to_completion(blocks: u64, net: &mut TimeTestNet) -> u64 {
	let mut wait_for = Vec::new();
	let highest_finalized = Arc::new(RwLock::new(0));
	for peer in &net.peers {
		let highest_finalized = highest_finalized.clone();
		let client = peer.client().clone();
		wait_for.push(Box::pin(
			client
				.finality_notification_stream()
				.take_while(move |n| {
					let mut highest_finalized = highest_finalized.write();
					if *n.header.number() > *highest_finalized {
						*highest_finalized = *n.header.number();
					}
					future::ready(n.header.number() < &blocks)
				})
				.collect::<Vec<_>>()
				.map(|_| ()),
		));
	}
	let wait_for = futures::future::join_all(wait_for);
	let drive_to_completion = futures::future::poll_fn(|cx| {
		net.poll(cx);
		Poll::<()>::Pending
	});
	future::select(wait_for, drive_to_completion).await;
	let highest_finalized = *highest_finalized.read();
	highest_finalized
}

#[tokio::test]
async fn finalize_3_voters_no_observers() {
	sp_tracing::try_init_simple();
	let mut net = TimeTestNet::default();
	tokio::spawn(initialize_grandpa(&mut net));
	net.peer(0).push_blocks(20, false);
	net.run_until_sync().await;
	for i in 0..3 {
		assert_eq!(net.peer(i).client().info().best_number, 20, "Peer #{i} failed to sync");
	}
	let hashof32 = net.peer(0).client().info().best_hash;
	run_to_completion(20, &mut net).await;
	// normally there's no justification for finalized blocks
	assert!(
		net.peer(0).client().justifications(hashof32).unwrap().is_none(),
		"Extra justification for block#1",
	);
}

#[tokio::test]
#[ignore]
async fn time_keygen_completes() {
	sp_tracing::init_for_tests();

	sp_tracing::info!(
		target: "time_keygen_completes",
		"Starting test..."
	);

	let peers = [TimeKeyring::Alice, TimeKeyring::Bob, TimeKeyring::Charlie];
	let mut net = TimeTestNet::default();
	let api = net.test_net.clone();

	let mut senders = vec![];
	let mut receivers = vec![];
	for _ in 0..peers.len() {
		let (s, r) = futures::channel::mpsc::channel(10);
		senders.push(s);
		receivers.push(r);
	}

	receivers.reverse();
	let time_peers = peers
		.iter()
		.enumerate()
		.map(|(id, p)| (id, p, api.clone(), receivers.pop().unwrap()))
		.collect::<Vec<_>>();

	tokio::spawn(initialize_grandpa(&mut net));
	tokio::spawn(initialize_time_worker(&mut net, time_peers));
	// let's wait for workers to properly spawn
	tokio::time::sleep(std::time::Duration::from_secs(12)).await;
	// Pushing 1 block
	net.peer(0).push_blocks(1, false);
	net.run_until_sync().await;

	// Verify all peers synchronized
	for i in 0..3 {
		assert_eq!(net.peer(i).client().info().best_number, 1, "Peer #{i} failed to sync");
	}

	run_to_completion(1, &mut net).await;

	for i in 0..3 {
		assert_eq!(net.peer(i).client().info().finalized_number, 1, "Peer #{i} failed to finalize");
	}

	tokio::time::sleep(std::time::Duration::from_secs(6)).await;
	assert!(!api.runtime_api().group_keys.lock().is_empty());

	// signing some data
	let message = [1u8; 32];
	for sender in &mut senders {
		assert!(sender.try_send((1, 1, 1, message)).is_ok());
	}
	tokio::time::sleep(std::time::Duration::from_secs(6)).await;
	assert!(!api.runtime_api().stored_signatures.lock().is_empty());
}
