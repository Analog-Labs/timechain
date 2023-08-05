#![allow(clippy::type_complexity)]
use crate::{communication::validator::topic, TW_LOG};
use futures::{
	channel::{mpsc, oneshot},
	FutureExt, StreamExt,
};
use log::{debug, error, warn};
use sc_client_api::{Backend, BlockchainEvents};
use sc_network_gossip::GossipEngine;
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_core::{sr25519, Pair};
use sp_keystore::KeystorePtr;
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
use time_primitives::{
	OcwPayload, ScheduleCycle, ShardId, TaskId, TimeApi, TssSignature, TIME_KEY_TYPE,
};
use tokio::time::Sleep;
use tss::{Timeout, Tss, TssAction, TssMessage};

pub type TssId = (TaskId, ScheduleCycle);

pub struct TssRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<TssSignature>,
}

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: ShardId,
	sender: sr25519::Public,
	payload: TssMessage<TssId>,
}

impl TimeMessage {
	fn encode(&self, kv: &KeystorePtr) -> Vec<u8> {
		let mut bytes = bincode::serialize(self).unwrap();
		let sig = kv.sr25519_sign(TIME_KEY_TYPE, &self.sender, &bytes).unwrap().unwrap();
		bytes.extend_from_slice(sig.as_ref());
		bytes
	}

	fn decode(bytes: &[u8]) -> Result<Self, ()> {
		if bytes.len() < 64 {
			return Err(());
		}
		let split = bytes.len() - 64;
		let mut sig = [0; 64];
		sig.copy_from_slice(&bytes[split..]);
		let payload = &bytes[..split];
		let msg: Self = bincode::deserialize(payload).map_err(|_| ())?;
		let sig = sr25519::Signature::from_raw(sig);
		if !sr25519::Pair::verify(&sig, payload, &msg.sender) {
			return Err(());
		}
		Ok(msg)
	}
}

struct TssState {
	tss: Tss<TssId, sr25519::Public>,
}

impl TssState {
	fn new(public: sr25519::Public) -> Self {
		Self { tss: Tss::new(public) }
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

pub struct WorkerParams<B: Block, BE, R> {
	pub _block: PhantomData<B>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_engine: GossipEngine<B>,
	pub kv: KeystorePtr,
	pub sign_data_receiver: mpsc::Receiver<TssRequest>,
}

/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<B: Block, BE, R> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	gossip_engine: GossipEngine<B>,
	kv: KeystorePtr,
	sign_data_receiver: mpsc::Receiver<TssRequest>,
	tss_states: HashMap<ShardId, TssState>,
	timeouts: HashMap<(ShardId, Option<TssId>), TssTimeout>,
	timeout: Option<Pin<Box<Sleep>>>,
	message_map: HashMap<ShardId, VecDeque<TimeMessage>>,
	requests: HashMap<TssId, oneshot::Sender<TssSignature>>,
}

impl<B, BE, R> TimeWorker<B, BE, R>
where
	B: Block + 'static,
	BE: Backend<B> + 'static,
	R: BlockchainEvents<B> + ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, BE, R>) -> Self {
		let WorkerParams {
			_block,
			backend,
			runtime,
			gossip_engine,
			kv,
			sign_data_receiver,
		} = worker_params;
		TimeWorker {
			_block,
			backend,
			runtime,
			gossip_engine,
			kv,
			sign_data_receiver,
			tss_states: Default::default(),
			timeouts: Default::default(),
			timeout: None,
			message_map: Default::default(),
			requests: Default::default(),
		}
	}

	/// Returns the public key for the worker if one was set.
	fn public_key(&self) -> Option<sr25519::Public> {
		let keys = self.kv.sr25519_public_keys(TIME_KEY_TYPE);
		if keys.is_empty() {
			warn!(target: TW_LOG, "No time key found, please inject one.");
			return None;
		}
		Some(keys[0])
	}

	/// On each grandpa finality we're initiating gossip to all other authorities to acknowledge
	fn on_finality(&mut self, block: <B as Block>::Hash, public_key: sr25519::Public) {
		log::info!("finality notification for {}", block);
		let shards = self.runtime.runtime_api().get_shards(block, public_key.into()).unwrap();
		debug!(target: TW_LOG, "Read shards from runtime {:?}", shards);
		for shard_id in shards {
			if self.tss_states.get(&shard_id).filter(|val| val.tss.is_initialized()).is_some() {
				debug!(target: TW_LOG, "Already participating in keygen for shard {}", shard_id);
				continue;
			}
			let members = self.runtime.runtime_api().get_shard_members(block, shard_id).unwrap();
			debug!(target: TW_LOG, "Participating in new keygen for shard {}", shard_id);
			let threshold = members.len() as _;
			let members =
				members.into_iter().map(|id| sr25519::Public::from_raw(id.into())).collect();
			let state =
				self.tss_states.entry(shard_id).or_insert_with(|| TssState::new(public_key));
			state.tss.initialize(members, threshold);
			self.poll_actions(shard_id, public_key);

			let Some(msg_queue) = self.message_map.remove(&shard_id) else {
				continue;
			};
			for msg in msg_queue {
				//wont fail since in first loop we already create a state and iterating that shard_id
				let tss_state = self.tss_states.get_mut(&shard_id).unwrap();
				tss_state.tss.on_message(msg.sender, msg.payload);
				self.poll_actions(shard_id, public_key);
			}
		}
	}

	fn poll_actions(&mut self, shard_id: u64, public_key: sr25519::Public) {
		let tss_state = self.tss_states.get_mut(&shard_id).unwrap();
		while let Some(action) = tss_state.tss.next_action() {
			match action {
				TssAction::Send(payload) => {
					debug!(target: TW_LOG, "Sending gossip message");
					let msg = TimeMessage {
						shard_id,
						sender: public_key,
						payload,
					};
					let bytes = msg.encode(&self.kv);
					self.gossip_engine.gossip_message(topic::<B>(), bytes, false);
				},
				TssAction::PublicKey(tss_public_key) => {
					let public_key = tss_public_key.to_bytes();
					log::info!("New group key provided: {:?} for id: {}", public_key, shard_id);
					self.timeouts.remove(&(shard_id, None));
					time_primitives::write_message(
						self.backend.offchain_storage().unwrap(),
						&OcwPayload::SubmitTssPublicKey { shard_id, public_key },
					);
				},
				TssAction::Tss(tss_signature, request_id) => {
					debug!(target: TW_LOG, "Storing tss signature");
					self.timeouts.remove(&(shard_id, Some(request_id)));
					let tss_signature = tss_signature.to_bytes();
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
		let mut gossips = self.gossip_engine.messages_for(topic::<B>());
		let mut finality_notifications = self.runtime.finality_notification_stream();
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
				_ = &mut self.gossip_engine => {
					error!(
						target: TW_LOG,
						"Gossip engine has terminated."
					);
					return;
				},
				notification = finality_notifications.next().fuse() => {
					let Some(notification) = notification else {
						debug!(
							target: TW_LOG,
							"no new finality notifications"
						);
						continue;
					};
					let Some(public_key) = self.public_key() else {
						continue;
					};
					self.on_finality(notification.header.hash(), public_key);
				},
				new_sig = self.sign_data_receiver.next().fuse() => {
					let Some(TssRequest { request_id, shard_id, data, tx }) = new_sig else {
						continue;
					};
					let Some(public_key) = self.public_key() else {
						continue;
					};
					let Some(tss_state) = self.tss_states.get_mut(&shard_id) else {
						continue;
					};
					self.requests.insert(request_id, tx);
					tss_state.tss.sign(request_id, data.to_vec());
					self.poll_actions(shard_id, public_key);
				},
				gossip = gossips.next().fuse() => {
					let Some(notification) = gossip else {
						debug!(target: TW_LOG, "no new gossip");
						continue;
					};
					let Some(public_key) = self.public_key() else {
						continue;
					};
					if let Ok(TimeMessage { shard_id, sender, payload }) = TimeMessage::decode(&notification.message){
						debug!(target: TW_LOG, "received gossip message {}", payload);
						if let Some(tss_state) = self.tss_states.get_mut(&shard_id) {
							tss_state.tss.on_message(sender, payload);
							self.poll_actions(shard_id, public_key);
						} else {
							log::info!("state not found, adding message in map with id {:?}", shard_id);
							self.message_map.entry(shard_id).or_default().push_back(TimeMessage { shard_id, sender, payload });
						}
					} else {
						debug!(target: TW_LOG, "received invalid message");
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
						let tss_state = self.tss_states.get_mut(&shard_id);
						if let (Some(tss_state), Some(timeout)) = (tss_state, timeout) {
							tss_state.tss.on_timeout(timeout.timeout);
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
