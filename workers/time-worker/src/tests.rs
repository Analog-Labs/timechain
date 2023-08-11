use crate::TimeWorkerParams;
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use sc_network_test::{Block, FullPeerConfig, TestNet, TestNetFactory};
use sp_api::{ApiRef, ProvideRuntimeApi};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use time_primitives::{PeerId, ShardId, TimeApi, TssRequest};

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
	}

}

impl ProvideRuntimeApi<Block> for MockApi {
	type Api = Self;

	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.clone().into()
	}
}

#[tokio::test]
async fn tss_smoke() -> Result<()> {
	sp_tracing::init_for_tests();

	sp_tracing::info!(
		target: "time_keygen_completes",
		"Starting test..."
	);

	let mut net = TestNet::default();
	let api = Arc::new(MockApi::default());
	let mut peers = vec![];
	let mut tss = vec![];
	for i in 0..3 {
		let (protocol_tx, protocol_rx) = async_channel::unbounded();
		let (tss_tx, tss_rx) = mpsc::channel(10);
		net.add_full_peer_with_config(FullPeerConfig {
			request_response_protocols: vec![crate::protocol_config(protocol_tx)],
			..Default::default()
		});
		// TODO: get real peer id
		peers.push(PeerId::default());
		tss.push(tss_tx);
		tokio::task::spawn(crate::start_timeworker_gadget(TimeWorkerParams {
			_block: PhantomData,
			backend: net.peer(i).client().as_backend(),
			client: net.peer(i).client().as_client(),
			runtime: api.clone(),
			network: net.peer(i).network_service().clone(),
			peer_id: peers[i],
			tss_request: tss_rx,
			protocol_request: protocol_rx,
		}));
	}

	api.create_shard(peers);

	net.peer(0).push_blocks(1, false);
	net.run_until_sync().await;

	// TODO: read public key from offchain storage

	// signing some data
	let message = [1u8; 32];
	for tss in &mut tss {
		let (tx, rx) = oneshot::channel();
		tss.send(TssRequest {
			request_id: (1, 1),
			shard_id: 1,
			data: message.to_vec(),
			tx,
		})
		.await?;
		rx.await?;
	}

	// TODO: read signature from offchain storage

	Ok(())
}
