#![allow(clippy::type_complexity)]
use crate::{PROTOCOL_NAME, TW_LOG};
use anyhow::Result;
use futures::{
	channel::{mpsc, oneshot},
	FutureExt, StreamExt,
};
use sc_client_api::{Backend, BlockchainEvents};
use sc_network::config::{IncomingRequest, OutgoingResponse};
use sc_network::{IfDisconnected, NetworkRequest, PeerId};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Block, Header};
use std::{
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	sync::Arc,
};
use time_primitives::{OcwPayload, ShardId, TimeApi, TssId, TssRequest, TssSignature};
use tss::{Tss, TssAction, TssMessage};

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: ShardId,
	block_number: u64,
	payload: TssMessage<TssId>,
}

impl TimeMessage {
	fn encode(&self) -> Vec<u8> {
		bincode::serialize(self).unwrap()
	}

	fn decode(bytes: &[u8]) -> Result<Self> {
		Ok(bincode::deserialize(bytes)?)
	}
}

pub(crate) fn to_peer_id(peer_id: time_primitives::PeerId) -> PeerId {
	PeerId::from_public_key(
		&sc_network::config::ed25519::PublicKey::try_from_bytes(&peer_id).unwrap().into(),
	)
}

pub struct WorkerParams<B: Block, BE, C, R, N> {
	pub _block: PhantomData<B>,
	pub backend: Arc<BE>,
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub network: N,
	pub peer_id: time_primitives::PeerId,
	pub tss_request: mpsc::Receiver<TssRequest>,
	pub protocol_request: async_channel::Receiver<IncomingRequest>,
}

/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<B: Block, BE, C, R, N> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	client: Arc<C>,
	runtime: Arc<R>,
	network: N,
	peer_id: time_primitives::PeerId,
	tss_request: mpsc::Receiver<TssRequest>,
	protocol_request: async_channel::Receiver<IncomingRequest>,
	tss_states: HashMap<ShardId, Tss<TssId, PeerId>>,
	messages: BTreeMap<u64, Vec<(ShardId, PeerId, TssMessage<TssId>)>>,
	requests: HashMap<TssId, oneshot::Sender<TssSignature>>,
}

impl<B, BE, C, R, N> TimeWorker<B, BE, C, R, N>
where
	B: Block + 'static,
	C: BlockchainEvents<B> + 'static,
	BE: Backend<B> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B>,
	N: NetworkRequest,
{
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, R, N>) -> Self {
		let WorkerParams {
			_block,
			backend,
			client,
			runtime,
			network,
			peer_id,
			tss_request,
			protocol_request,
		} = worker_params;
		Self {
			_block,
			backend,
			client,
			runtime,
			network,
			peer_id,
			tss_request,
			protocol_request,
			tss_states: Default::default(),
			messages: Default::default(),
			requests: Default::default(),
		}
	}

	fn on_finality(&mut self, block: <B as Block>::Hash, block_number: u64) {
		let local_peer_id = to_peer_id(self.peer_id);
		log::debug!(target: TW_LOG, "{}: on_finality {}", local_peer_id, block.to_string());
		let shards = self.runtime.runtime_api().get_shards(block, self.peer_id).unwrap();
		for shard_id in shards {
			if self.tss_states.get(&shard_id).is_some() {
				continue;
			}
			let members = self.runtime.runtime_api().get_shard_members(block, shard_id).unwrap();
			log::debug!(target: TW_LOG, "shard {}: {} joining shard", shard_id, local_peer_id);
			let threshold = members.len() as _;
			let members = members.into_iter().map(to_peer_id).collect();
			self.tss_states.insert(shard_id, Tss::new(local_peer_id, members, threshold));
			self.poll_actions(shard_id, block_number);
		}
		while let Some(n) = self.messages.keys().copied().next() {
			if n > block_number {
				break;
			}
			for (shard_id, peer_id, msg) in self.messages.remove(&n).unwrap() {
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				log::error!("dropping message {} {} {}", shard_id, peer_id, msg);
				continue;
			};
				tss.on_message(peer_id, msg);
				self.poll_actions(shard_id, n);
			}
		}
	}

	fn poll_actions(&mut self, shard_id: ShardId, block_number: u64) {
		let tss = self.tss_states.get_mut(&shard_id).unwrap();
		while let Some(action) = tss.next_action() {
			match action {
				TssAction::Send(msgs) => {
					let local_peer_id = to_peer_id(self.peer_id);
					for (peer, payload) in msgs {
						log::debug!(
							target: TW_LOG,
							"shard {}: {} tx {} to {}",
							shard_id,
							local_peer_id,
							payload,
							peer
						);
						let msg = TimeMessage {
							shard_id,
							block_number,
							payload,
						};
						let bytes = msg.encode();
						let (tx, rx) = oneshot::channel();
						self.network.start_request(
							peer,
							PROTOCOL_NAME.into(),
							bytes,
							tx,
							IfDisconnected::TryConnect,
						);
						tokio::task::spawn(async move {
							if let Ok(Err(err)) = rx.await {
								log::error!(
									target: TW_LOG,
									"shard {}: {} tx {} to {} network error {}",
									shard_id,
									local_peer_id,
									msg.payload,
									peer,
									err,
								);
							}
						});
					}
				},
				TssAction::PublicKey(tss_public_key) => {
					let public_key = tss_public_key.to_bytes();
					log::info!(target: TW_LOG, "shard {}: public key {:?}", shard_id, public_key);
					time_primitives::write_message(
						self.backend.offchain_storage().unwrap(),
						&OcwPayload::SubmitTssPublicKey { shard_id, public_key },
					);
				},
				TssAction::Signature(request_id, tss_signature) => {
					let tss_signature = tss_signature.to_bytes();
					log::debug!(
						target: TW_LOG,
						"shard {}: req {:?}: signature {:?}",
						shard_id,
						request_id,
						tss_signature
					);
					if let Some(tx) = self.requests.remove(&request_id) {
						tx.send(tss_signature).ok();
					}
				},
				TssAction::Error(id, peer, error) => {
					log::error!("{:?} {:?} {:?}", id, peer, error);
					time_primitives::write_message(
						self.backend.offchain_storage().unwrap(),
						&OcwPayload::SetShardOffline { shard_id },
					);
				},
			}
		}
	}

	/// Our main worker main process - we act on grandpa finality and gossip messages for interested
	/// topics
	pub(crate) async fn run(&mut self) {
		let mut finality_notifications = self.client.finality_notification_stream();
		loop {
			futures::select! {
				notification = finality_notifications.next().fuse() => {
					let Some(notification) = notification else {
						log::debug!(
							target: TW_LOG,
							"no new finality notifications"
						);
						continue;
					};
					let block_hash = notification.header.hash();
					let block_number = notification.header.number().to_string().parse().unwrap();
					self.on_finality(block_hash, block_number);
				},
				tss_request = self.tss_request.next().fuse() => {
					let Some(TssRequest { request_id, shard_id, block_number, data, tx }) = tss_request else {
						continue;
					};
					let Some(tss) = self.tss_states.get_mut(&shard_id) else {
						continue;
					};
					log::debug!(target: TW_LOG, "shard {}: req {:?}: sign", shard_id, request_id);
					self.requests.insert(request_id, tx);
					tss.sign(request_id, data.to_vec());
					self.poll_actions(shard_id, block_number);
				},
				protocol_request = self.protocol_request.next().fuse() => {
					let Some(IncomingRequest { peer, payload, pending_response }) = protocol_request else {
						continue;
					};
					let _ = pending_response.send(OutgoingResponse {
						result: Ok(vec![]),
						reputation_changes: vec![],
						sent_feedback: None,
					});
					if let Ok(TimeMessage { shard_id, block_number, payload }) = TimeMessage::decode(&payload) {
						let local_peer_id = to_peer_id(self.peer_id);
						log::debug!(target: TW_LOG, "shard {}: {} rx {} from {}",
							shard_id, local_peer_id, payload, peer);
						if let Some(tss) = self.tss_states.get_mut(&shard_id) {
							tss.on_message(peer, payload);
							self.poll_actions(shard_id, block_number);
						} else {
							log::info!(target: TW_LOG, "state not found, adding message in map with id {:?}", shard_id);
							self.messages.entry(block_number).or_default().push((shard_id, peer, payload));
						}
					} else {
						log::debug!(target: TW_LOG, "received invalid message");
						continue;
					}
				},
			}
		}
	}
}
