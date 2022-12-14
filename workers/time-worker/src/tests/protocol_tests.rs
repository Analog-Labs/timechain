use crate::{
	start_timeworker_gadget, tests::kv_tests::Keyring as TimeKeyring, traits::Client,
	TimeWorkerParams,
};
use arrayref::array_ref;
use codec::{Codec, Decode, Encode};
use futures::{future, stream::FuturesUnordered, Future, FutureExt, StreamExt};
use parking_lot::{Mutex, RwLock};
use sc_client_api::Backend;
use sc_consensus::BoxJustificationImport;
use sc_finality_grandpa::{
	run_grandpa_voter, Config, GenesisAuthoritySetProvider, GrandpaParams, LinkHalf,
	SharedVoterState,
};
use sc_keystore::LocalKeystore;
use sc_network::config::Role;
use sc_network_common::request_responses::ProtocolConfig;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, Hash, PassThroughVerifier, Peer, PeersClient,
	PeersFullClient, TestNetFactory,
};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_api::{ApiRef, BlockId, ProvideRuntimeApi};
use sp_application_crypto::Pair as SPPair;
use sp_consensus::BlockOrigin;
use sp_core::{crypto::key_types::GRANDPA, sr25519::Pair};
use sp_finality_grandpa::{
	AuthorityList, EquivocationProof, GrandpaApi, OpaqueKeyOwnershipProof, SetId,
};
use sp_runtime::{traits::Header as HeaderT, BuildStorage, DigestItem};
use std::{
	marker::PhantomData,
	pin::Pin,
	sync::{Arc, Mutex as StdMutex},
	task::Poll,
	thread::sleep,
	time::Duration,
};
use substrate_test_runtime_client::{
	runtime::Header, Ed25519Keyring, LongestChain, SyncCryptoStore, SyncCryptoStorePtr,
};
use time_primitives::{
	crypto::{Public as TimeKey, Signature},
	TimeApi, KEY_TYPE as TimeKeyType,
};
use tokio::runtime::{Handle, Runtime};

// required for test networking
const TIME_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"TIME";

#[derive(Decode, Encode, TypeInfo)]
pub enum ConsensusLog<Public: Codec> {
	#[codec(index = 1)]
	AuthoritiesChange(Vec<Public>),
}

type TestLinkHalf =
	LinkHalf<Block, PeersFullClient, LongestChain<substrate_test_runtime_client::Backend, Block>>;
type GrandpaPeerData = Mutex<Option<TestLinkHalf>>;
type GrandpaBlockImport = sc_finality_grandpa::GrandpaBlockImport<
	substrate_test_runtime_client::Backend,
	Block,
	PeersFullClient,
	LongestChain<substrate_test_runtime_client::Backend, Block>,
>;
type GrandpaPeer = Peer<GrandpaPeerData, GrandpaBlockImport>;

pub(crate) struct TimeTestNet {
	peers: Vec<GrandpaPeer>,
	test_net: Arc<TestApi>,
}

// same as runtime
pub(crate) type BlockNumber = u32;
pub(crate) type GrandpaBlockNumber = u64;

const TIME_PROTOCOL_NAME: &str = "/time/1";
const GRANDPA_PROTOCOL_NAME: &str = "/paritytech/grandpa/1";
const TEST_GOSSIP_DURATION: Duration = Duration::from_millis(500);

impl TimeTestNet {
	pub(crate) fn new(n_authority: usize, n_full: usize, test_net: Arc<TestApi>) -> Self {
		let capacity = n_authority + n_full;
		let mut net = TimeTestNet { peers: Vec::with_capacity(capacity), test_net };
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
			notifications_protocols: vec![GRANDPA_PROTOCOL_NAME.into(), TIME_PROTOCOL_NAME.into()],
			is_authority: true,
			..Default::default()
		})
	}

	pub(crate) fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![GRANDPA_PROTOCOL_NAME.into(), TIME_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}
	pub(crate) fn generate_blocks(
		&mut self,
		count: usize,
		session_length: u64,
		validator_set: &Vec<TimeKey>,
	) {
		self.peer(0).generate_blocks(count, BlockOrigin::File, |builder| {
			let mut block = builder.build().unwrap().block;

			if *block.header.number() % session_length == 0 {
				add_auth_change_digest(&mut block.header, validator_set.clone());
			}

			block
		});
	}
	pub(crate) fn drop_last_worker(&mut self) {
		self.peers.pop();
	}
}

impl Default for TimeTestNet {
	fn default() -> Self {
		todo!()
	}
}

impl TestNetFactory for TimeTestNet {
	type Verifier = PassThroughVerifier;
	type BlockImport = GrandpaBlockImport;
	type PeerData = GrandpaPeerData;

	fn make_verifier(&self, _client: PeersClient, _: &GrandpaPeerData) -> Self::Verifier {
		PassThroughVerifier::new(false) // use non-instant finality.
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

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<BoxJustificationImport<Block>>,
		GrandpaPeerData,
	) {
		let (client, backend) = (client.as_client(), client.as_backend());
		let (import, link) = sc_finality_grandpa::block_import(
			client.clone(),
			&*self.test_net,
			LongestChain::new(backend.clone()),
			None,
		)
		.expect("Could not create block import for fresh peer.");
		let justification_import = Box::new(import.clone());
		(BlockImportAdapter::new(import), Some(justification_import), Mutex::new(Some(link)))
	}

	fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![GRANDPA_PROTOCOL_NAME.into(), TIME_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}
}

fn add_auth_change_digest(header: &mut Header, new_auth_set: Vec<TimeKey>) {
	header.digest_mut().push(DigestItem::Consensus(
		TIME_ENGINE_ID,
		ConsensusLog::<TimeKey>::AuthoritiesChange(new_auth_set).encode(),
	));
}

#[derive(Serialize, Deserialize, Debug)]
struct Genesis(std::collections::BTreeMap<String, String>);

impl BuildStorage for Genesis {
	fn assimilate_storage(&self, storage: &mut sp_core::storage::Storage) -> Result<(), String> {
		storage
			.top
			.extend(self.0.iter().map(|(a, b)| (a.clone().into_bytes(), b.clone().into_bytes())));
		Ok(())
	}
}

fn make_time_ids(keys: &[TimeKeyring]) -> Vec<TimeKey> {
	keys.iter().map(|key| Pair::from(key.clone()).public().into()).collect()
}

pub(crate) fn create_time_keystore(authority: TimeKeyring) -> SyncCryptoStorePtr {
	let keystore = Arc::new(LocalKeystore::in_memory());
	SyncCryptoStore::sr25519_generate_new(&*keystore, TimeKeyType, Some(&authority.to_seed()))
		.expect("Creates authority key");
	keystore
}

#[derive(Default, Clone)]
pub(crate) struct TestApi {
	genesys_validator_set: Vec<TimeKeyring>,
	next_validator_set: Vec<TimeKeyring>,
	genesys_authorities: AuthorityList,
}

// compiler gets confused and warns us about unused inner
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct RuntimeApi {
	inner: TestApi,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = RuntimeApi;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		RuntimeApi { inner: self.clone() }.into()
	}
}

impl TestApi {
	fn authority_list(&self) -> AuthorityList {
		use sp_application_crypto::RuntimeAppPublic;
		self.genesys_validator_set
			.clone()
			.into_iter()
			.map(|k| {
				let key_vec = k.public().to_raw_vec();
				let key = array_ref!(key_vec, 0, 32);
				(sp_application_crypto::ed25519::Public::from_raw(*key).into(), 1u64)
			})
			.collect()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl GrandpaApi<Block> for RuntimeApi {
		fn grandpa_authorities(&self) -> AuthorityList {
			self.inner.genesys_authorities.clone()
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
			_authority_id: sp_finality_grandpa::AuthorityId,
		) -> Option<OpaqueKeyOwnershipProof> {
			None
		}
	}

	impl TimeApi<Block> for RuntimeApi {
		fn store_signature(auth_key: time_primitives::TimeId, auth_sig: time_primitives::TimeSignature, signature_data: time_primitives::SignatureData, network_id: Vec<u8>, block_height: u64,) {}
	}

}

impl GenesisAuthoritySetProvider<Block> for TestApi {
	fn get(&self) -> sp_blockchain::Result<AuthorityList> {
		Ok(self.authority_list())
	}
}

fn create_keystore(authority: Ed25519Keyring) -> (SyncCryptoStorePtr, tempfile::TempDir) {
	let keystore_path = tempfile::tempdir().expect("Creates keystore path");
	let keystore =
		Arc::new(LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore"));
	SyncCryptoStore::ed25519_generate_new(&*keystore, GRANDPA, Some(&authority.to_seed()))
		.expect("Creates authority key");

	(keystore, keystore_path)
}

// Spawns grandpa voters. Returns a future to spawn on the runtime.
fn initialize_grandpa(
	net: &mut TimeTestNet,
	grandpa_peers: &[Ed25519Keyring],
) -> impl Future<Output = ()> {
	let voters = FuturesUnordered::new();

	// initializing grandpa gadget per peer
	for (peer_id, key) in grandpa_peers.iter().enumerate() {
		let (keystore, _) = create_keystore(*key);

		let (net_service, link) = {
			// temporary needed for some reason
			let link =
				net.peers[peer_id].data.lock().take().expect("link initialized at startup; qed");
			(net.peers[peer_id].network_service().clone(), link)
		};

		let grandpa_params = GrandpaParams {
			config: Config {
				gossip_duration: TEST_GOSSIP_DURATION,
				justification_period: 32,
				keystore: Some(keystore),
				name: Some(format!("peer#{}", peer_id)),
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
	peers: Vec<(usize, &TimeKeyring, Arc<API>)>,
) -> impl Future<Output = ()>
where
	API: ProvideRuntimeApi<Block> + Send + Sync + Default,
	API::Api: TimeApi<Block>,
{
	let time_workers = FuturesUnordered::new();

	// initializing time gadget per peer
	for (peer_id, key, api) in peers.into_iter() {
		let peer = &net.peers[peer_id];

		let keystore = create_time_keystore(*key);

		let time_params = TimeWorkerParams {
			client: peer.client().as_client(),
			backend: peer.client().as_backend(),
			runtime: api.clone(),
			gossip_network: peer.network_service().clone(),
			kv: Some(keystore).into(),
			_block: PhantomData::default(),
		};
		let gadget = start_timeworker_gadget::<_, _, _, _, _>(time_params);

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&gadget);
		time_workers.push(gadget);
	}

	time_workers.for_each(|_| async move {})
}

fn block_until_complete(
	future: impl Future + Unpin,
	net: &Arc<Mutex<TimeTestNet>>,
	runtime: &mut Runtime,
) {
	let drive_to_completion = futures::future::poll_fn(|cx| {
		net.lock().poll(cx);
		Poll::<()>::Pending
	});
	runtime.block_on(future::select(future, drive_to_completion));
}

// run the voters to completion. provide a closure to be invoked after
// the voters are spawned but before blocking on them.
fn run_to_completion_with<F>(
	runtime: &mut Runtime,
	blocks: u64,
	net: Arc<Mutex<TimeTestNet>>,
	peers: &[Ed25519Keyring],
	with: F,
) -> u64
where
	F: FnOnce(Handle) -> Option<Pin<Box<dyn Future<Output = ()>>>>,
{
	let mut wait_for = Vec::new();

	let highest_finalized = Arc::new(RwLock::new(0));

	if let Some(f) = (with)(runtime.handle().clone()) {
		wait_for.push(f);
	};

	for (peer_id, _) in peers.iter().enumerate() {
		let highest_finalized = highest_finalized.clone();
		let client = net.lock().peers[peer_id].client().clone();

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

	// wait for all finalized on each.
	let wait_for = ::futures::future::join_all(wait_for);

	block_until_complete(wait_for, &net, runtime);
	let highest_finalized = *highest_finalized.read();
	highest_finalized
}

fn run_to_completion(
	runtime: &mut Runtime,
	blocks: u64,
	net: Arc<Mutex<TimeTestNet>>,
	peers: &[Ed25519Keyring],
) -> u64 {
	run_to_completion_with(runtime, blocks, net, peers, |_| None)
}

fn make_gradpa_ids(keys: &[Ed25519Keyring]) -> AuthorityList {
	keys.iter().map(|key| key.clone().public().into()).map(|id| (id, 1)).collect()
}

#[test]
fn time_keygen_completes() {
	sp_tracing::try_init_simple();

	sp_tracing::info!(
		target: "time_keygen_completes",
		"Starting test..."
	);
	// our runtime for the test chain
	let mut runtime = Runtime::new().unwrap();

	// creating 3 validators
	let peers = &[TimeKeyring::Alice, TimeKeyring::Bob, TimeKeyring::Charlie];
	let grandpa_peers = &[Ed25519Keyring::Alice, Ed25519Keyring::Bob, Ed25519Keyring::Charlie];
	let genesys_authorities = make_gradpa_ids(grandpa_peers);

	let validator_set = make_time_ids(peers);
	let test_api = Arc::new(TestApi {
		genesys_validator_set: vec![TimeKeyring::Alice, TimeKeyring::Bob, TimeKeyring::Charlie],
		next_validator_set: vec![TimeKeyring::Alice, TimeKeyring::Charlie, TimeKeyring::Dave],
		genesys_authorities,
	});

	// our time network with 3 authorities and 1 full peer
	let mut network = TimeTestNet::new(3, 1, test_api.clone());
	let time_peers = peers
		.iter()
		.enumerate()
		.map(|(id, p)| (id, p, test_api.clone()))
		.collect::<Vec<_>>();

	runtime.spawn(initialize_grandpa(&mut network, grandpa_peers));
	runtime.spawn(initialize_time_worker(&mut network, time_peers));

	// Pushing 20 block
	network.generate_blocks(20, 10, &validator_set);
	network.block_until_sync();

	// Verify all peers synchronized
	for i in 0..3 {
		assert_eq!(network.peer(i).client().info().best_number, 20, "Peer #{} failed to sync", i);
	}

	let net = Arc::new(Mutex::new(network));

	run_to_completion(&mut runtime, 20, net.clone(), grandpa_peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			20,
			"Peer #{} failed to finalize",
			i
		);
	}

	// we need this as otherwise we'll end up checking storage before actual work is done in async
	// tasks
	sleep(Duration::from_secs(300));
}
