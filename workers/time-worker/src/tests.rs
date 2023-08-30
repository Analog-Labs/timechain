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
use sp_runtime::traits::Extrinsic;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;
use time_primitives::{
	PeerId, ShardId, TimeApi, TssId, TssPublicKey, TssRequest, TssSignature, TIME_KEY_TYPE,
};

#[derive(Default)]
struct InnerMockApi {
	shard_counter: ShardId,
	shards: HashMap<PeerId, Vec<ShardId>>,
	members: HashMap<ShardId, Vec<PeerId>>,
}

impl InnerMockApi {
	fn create_shard(&mut self, members: Vec<PeerId>) -> ShardId {
		let id = self.shard_counter;
		self.shard_counter += 1;
		for member in &members {
			self.shards.entry(*member).or_default().push(id);
		}
		self.members.insert(id, members);
		id
	}

	fn get_shards(&self, peer_id: PeerId) -> Vec<ShardId> {
		self.shards.get(&peer_id).cloned().unwrap_or_default()
	}

	fn get_shard_members(&self, shard_id: ShardId) -> Vec<PeerId> {
		self.members.get(&shard_id).cloned().unwrap_or_default()
	}
}

#[derive(Clone, Default)]
struct MockApi {
	inner: Arc<Mutex<InnerMockApi>>,
}

impl MockApi {
	pub fn create_shard(&self, members: Vec<PeerId>) -> ShardId {
		self.inner.lock().unwrap().create_shard(members)
	}
}

sp_api::mock_impl_runtime_apis! {
	impl TimeApi<Block> for MockApi {
		fn get_shards(&self, peer_id: PeerId) -> Vec<ShardId> {
			self.inner.lock().unwrap().get_shards(peer_id)
		}

		fn get_shard_members(shard_id: ShardId) -> Vec<PeerId> {
			self.inner.lock().unwrap().get_shard_members(shard_id)
		}

		fn get_shard_threshold(_shard_id: ShardId) -> u16 {
			3
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
	let public_key = frost_evm::VerifyingKey::from_bytes(public_key)?;
	let signature = frost_evm::Signature::from_bytes(signature)?;
	public_key.verify(message, &signature)?;
	Ok(())
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

	let client = Arc::new(substrate_test_runtime_client::new());
	let spawner = sp_core::testing::TaskExecutor::new();
	let txpool =
		BasicPool::new_full(Default::default(), true.into(), None, spawner.clone(), client.clone());
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
		}));
	}
	net.run_until_connected().await;

	let client: Vec<_> = (0..3).map(|i| net.peer(i).client().as_client()).collect();

	log::info!(
		"creating shard with members {:#?}",
		peers
			.iter()
			.map(|p| crate::worker::to_peer_id(*p).to_string())
			.collect::<Vec<_>>()
	);
	api.create_shard(peers);

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
	// let mut tss_public_key = None;
	sp_std::if_std! {
		println!("{:?}", txpool.status());
	}

	// for storage in &storage {
	// 	loop {
	// 		// let Some(msg) = time_primitives::read_message(storage.clone()) else {
	// 		// 	tokio::time::sleep(Duration::from_secs(1)).await;
	// 		// 	continue;
	// 		// };
	// 		// let OcwPayload::SubmitTssPublicKey { shard_id, public_key } = msg else {
	// 		// 	anyhow::bail!("unexpected msg {:?}", msg);
	// 		// };
	// 		assert_eq!(shard_id, 0);
	// 		tss_public_key = Some(public_key);
	// 		break;
	// 	}
	// }
	// let public_key = tss_public_key.unwrap();
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
	// for rx in rxs {
	// 	let signature = rx.await?;
	// 	verify_tss_signature(public_key, &message, signature)?;
	// }

	Ok(())
}
