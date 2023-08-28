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
	collections::{HashMap, VecDeque},
	future::Future,
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
	task::Poll,
	time::{Duration, Instant},
};
use time_primitives::{OcwPayload, ShardId, TimeApi, TssId, TssRequest, TssSignature};
use tokio::time::Sleep;
use tss::{Timeout, Tss, TssAction, TssMessage};

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: ShardId,
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

struct TssTimeout {
	timeout: Timeout<TssId>,
	deadline: Instant,
}

impl TssTimeout {
	fn new(timeout: Timeout<TssId>) -> Self {
		let deadline = Instant::now() + Duration::from_secs(30);
		Self { timeout, deadline }
	}
}

fn sleep_until(deadline: Instant) -> Pin<Box<Sleep>> {
	Box::pin(tokio::time::sleep_until(deadline.into()))
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
	timeouts: HashMap<(ShardId, Option<TssId>), TssTimeout>,
	timeout: Option<Pin<Box<Sleep>>>,
	message_map: HashMap<ShardId, VecDeque<(PeerId, TimeMessage)>>,
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
			timeouts: Default::default(),
			timeout: None,
			message_map: Default::default(),
			requests: Default::default(),
		}
	}

	fn on_finality(&mut self, block: <B as Block>::Hash) {
		let local_peer_id = to_peer_id(self.peer_id);
		log::debug!(target: TW_LOG, "{}: on_finality {}", local_peer_id, block.to_string());
		let shards = self.runtime.runtime_api().get_shards(block, self.peer_id).unwrap();
		for shard_id in shards {
			if self.tss_states.get(&shard_id).filter(|tss| tss.is_initialized()).is_some() {
				continue;
			}
			let members = self.runtime.runtime_api().get_shard_members(block, shard_id).unwrap();
			log::debug!(target: TW_LOG, "shard {}: {} joining shard", shard_id, local_peer_id);
			let threshold = members.len() as _;
			let members = members.into_iter().map(to_peer_id).collect();
			let tss = self.tss_states.entry(shard_id).or_insert_with(|| Tss::new(local_peer_id));
			tss.initialize(members, threshold);
			self.poll_actions(shard_id);

			let Some(msg_queue) = self.message_map.remove(&shard_id) else {
				continue;
			};
			for (peer_id, msg) in msg_queue {
				//wont fail since in first loop we already create a state and iterating that shard_id
				let tss = self.tss_states.get_mut(&shard_id).unwrap();
				tss.on_message(peer_id, msg.payload);
				self.poll_actions(shard_id);
			}
		}
	}

	fn poll_actions(&mut self, shard_id: ShardId) {
		let tss = self.tss_states.get_mut(&shard_id).unwrap();
		while let Some(action) = tss.next_action() {
			match action {
				TssAction::Send(peer, payload) => {
					let local_peer_id = to_peer_id(self.peer_id);
					log::debug!(
						target: TW_LOG,
						"shard {}: {} tx {} to {}",
						shard_id,
						local_peer_id,
						payload,
						peer
					);
					let msg = TimeMessage { shard_id, payload };
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
								"shard {}: {} tx {} to {} network error {:?}",
								shard_id,
								local_peer_id,
								msg.payload,
								peer,
								err,
							);
						}
					});
				},
				TssAction::PublicKey(tss_public_key) => {
					let public_key = tss_public_key.to_bytes();
					log::info!(target: TW_LOG, "shard {}: public key {:?}", shard_id, public_key);
					self.timeouts.remove(&(shard_id, None));
					time_primitives::write_message(
						self.backend.offchain_storage().unwrap(),
						&OcwPayload::SubmitTssPublicKey { shard_id, public_key },
					);
				},
				TssAction::Tss(tss_signature, request_id) => {
					let tss_signature = tss_signature.to_bytes();
					log::debug!(
						target: TW_LOG,
						"shard {}: req {:?}: signature {:?}",
						shard_id,
						request_id,
						tss_signature
					);
					self.timeouts.remove(&(shard_id, Some(request_id)));
					if let Some(tx) = self.requests.remove(&request_id) {
						tx.send(tss_signature).ok();
					}
				},
				TssAction::Report(_, hash) => {
					self.timeouts.remove(&(shard_id, hash));
				},
				TssAction::Timeout(timeout, hash) => {
					let timeout = TssTimeout::new(timeout);
					if self.timeout.is_none() {
						self.timeout = Some(sleep_until(timeout.deadline));
					}
					self.timeouts.insert((shard_id, hash), timeout);
				},
			}
		}
	}

	/// Our main worker main process - we act on grandpa finality and gossip messages for interested
	/// topics
	pub(crate) async fn run(&mut self) {
		let mut finality_notifications = self.client.finality_notification_stream();
		loop {
			let timeout = futures::future::poll_fn(|cx| {
				if let Some(timeout) = self.timeout.as_mut() {
					futures::pin_mut!(timeout);
					timeout.poll(cx)
				} else {
					Poll::Pending
				}
			});
			futures::select! {
				notification = finality_notifications.next().fuse() => {
					let Some(notification) = notification else {
						log::debug!(
							target: TW_LOG,
							"no new finality notifications"
						);
						continue;
					};
					log::debug!(target: TW_LOG, "finalized {}", notification.header.number());
					self.on_finality(notification.header.hash());
				},
				tss_request = self.tss_request.next().fuse() => {
					let Some(TssRequest { request_id, shard_id, data, tx }) = tss_request else {
						continue;
					};
					let Some(tss) = self.tss_states.get_mut(&shard_id) else {
						log::debug!(target: TW_LOG, "trying to run task on non existent shard {}", shard_id);
						continue;
					};
					log::debug!(target: TW_LOG, "shard {}: req {:?}: sign", shard_id, request_id);
					self.requests.insert(request_id, tx);
					tss.sign(request_id, data.to_vec());
					self.poll_actions(shard_id);
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
					if let Ok(TimeMessage { shard_id, payload }) = TimeMessage::decode(&payload) {
						let local_peer_id = to_peer_id(self.peer_id);
						log::debug!(target: TW_LOG, "shard {}: {} rx {} from {}",
							shard_id, local_peer_id, payload, peer);
						if let Some(tss) = self.tss_states.get_mut(&shard_id) {
							tss.on_message(peer, payload);
							self.poll_actions(shard_id);
						} else {
							log::info!(target: TW_LOG, "state not found, adding message in map with id {:?}", shard_id);
							self.message_map.entry(shard_id).or_default().push_back((peer, TimeMessage { shard_id, payload }));
						}
					} else {
						log::debug!(target: TW_LOG, "received invalid message");
						continue;
					}
				},
				_ = timeout.fuse() => {
					let mut next_timeout = None;
					let mut fired = vec![];
					let now = Instant::now();
					for (key, timeout) in &self.timeouts {
						if timeout.deadline <= now {
							fired.push(*key);
						} else if let Some(deadline) = next_timeout {
							if timeout.deadline < deadline {
								next_timeout = Some(timeout.deadline);
							}
						} else if next_timeout.is_none() {
							next_timeout = Some(timeout.deadline);
						}
					}
					for (shard_id, hash) in fired {
						let timeout = self.timeouts.remove(&(shard_id, hash));
						let tss = self.tss_states.get_mut(&shard_id);
						if let (Some(tss), Some(timeout)) = (tss, timeout) {
							tss.on_timeout(timeout.timeout);
						}
					}
					if let Some(next_timeout) = next_timeout {
						self.timeout = Some(sleep_until(next_timeout));
					}
				}
			}
		}
	}
}
