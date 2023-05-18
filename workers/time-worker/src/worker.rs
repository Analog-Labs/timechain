#![allow(clippy::type_complexity)]
use crate::{
	communication::validator::{topic, GossipValidator},
	Client, WorkerParams, TW_LOG,
};
use futures::{channel::mpsc, FutureExt, StreamExt};
use log::{debug, error, warn};
use sc_client_api::{Backend, FinalityNotification, FinalityNotifications};
use sc_network_gossip::GossipEngine;
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_core::{Encode, Decode};
use sp_core::{sr25519, Pair};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block, Header};
use std::{
	collections::HashMap,
	future::Future,
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
	task::Poll,
	time::{Duration, Instant},
};
use time_primitives::{abstraction::EthTxValidation, TimeApi, KEY_TYPE};
use tokio::time::Sleep;
use tss::{Timeout, Tss, TssAction, TssMessage};

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: u64,
	sender: sr25519::Public,
	payload: TssMessage,
}

impl TimeMessage {
	fn encode(&self, kv: &KeystorePtr) -> Vec<u8> {
		let mut bytes = bincode::serialize(self).unwrap();
		let sig = kv.sr25519_sign(KEY_TYPE, &self.sender, &bytes).unwrap().unwrap();
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

struct Shard {
	tss: Tss<sr25519::Public>,
	is_collector: bool,
}

impl Shard {
	fn new(public: sr25519::Public) -> Self {
		Self {
			tss: Tss::new(public),
			is_collector: false,
		}
	}
}

struct TssTimeout {
	timeout: Timeout,
	deadline: Instant,
}

impl TssTimeout {
	fn new(timeout: Timeout) -> Self {
		let deadline = Instant::now() + Duration::from_secs(30);
		Self { timeout, deadline }
	}
}

fn sleep_until(deadline: Instant) -> Pin<Box<Sleep>> {
	Box::pin(tokio::time::sleep_until(deadline.into()))
}

/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<B: Block, A, C, R, BE> {
	_client: Arc<C>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	kv: KeystorePtr,
	shards: HashMap<u64, Shard>,
	finality_notifications: FinalityNotifications<B>,
	gossip_engine: GossipEngine<B>,
	_gossip_validator: Arc<GossipValidator<B>>,
	sign_data_receiver: mpsc::Receiver<(u64, [u8; 32])>,
	tx_data_sender: mpsc::Sender<Vec<u8>>,
	gossip_data_receiver: mpsc::Receiver<Vec<u8>>,
	accountid: PhantomData<A>,
	timeouts: HashMap<(u64, Option<[u8; 32]>), TssTimeout>,
	timeout: Option<Pin<Box<Sleep>>>,
}

impl<B, A, C, R, BE> TimeWorker<B, A, C, R, BE>
where
	B: Block + 'static,
	A: sp_runtime::codec::Codec + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B, A>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, A, C, R, BE>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			gossip_engine,
			gossip_validator,
			kv,
			sign_data_receiver,
			tx_data_sender,
			gossip_data_receiver,
			accountid,
		} = worker_params;
		TimeWorker {
			finality_notifications: client.finality_notification_stream(),
			_client: client,
			backend,
			runtime,
			gossip_engine,
			_gossip_validator: gossip_validator,
			kv,
			sign_data_receiver,
			tx_data_sender,
			gossip_data_receiver,
			shards: Default::default(),
			accountid,
			timeouts: Default::default(),
			timeout: None,
		}
	}

	/// Returns the public key for the worker if one was set.
	fn public_key(&self) -> Option<sr25519::Public> {
		let keys = self.kv.sr25519_public_keys(KEY_TYPE);
		if keys.is_empty() {
			warn!(target: TW_LOG, "No time key found, please inject one.");
			return None;
		}
		Some(keys[0])
	}

	/// On each grandpa finality we're initiating gossip to all other authorities to acknowledge
	fn on_finality(&mut self, notification: FinalityNotification<B>, public_key: sr25519::Public) {
		let shards = self.runtime.runtime_api().get_shards(notification.header.hash()).unwrap();
		debug!(target: TW_LOG, "Read shards from runtime {:?}", shards);
		for (shard_id, shard) in shards {
			if self.shards.contains_key(&shard_id) {
				continue;
			}
			if !shard.members().contains(&public_key.into()) {
				debug!(target: TW_LOG, "Not a member of shard {}", shard_id);
				continue;
			}
			debug!(target: TW_LOG, "Participating in new keygen for shard {}", shard_id);

			let members = shard
				.members()
				.into_iter()
				.map(|id| sr25519::Public::from_raw(id.into()))
				.collect();
			let state = self.shards.entry(shard_id).or_insert_with(|| Shard::new(public_key));
			state.tss.initialize(members, shard.threshold());
			state.is_collector = *shard.collector() == public_key.into();
			self.poll_actions(shard_id, public_key);
		}
	}

	fn poll_actions(&mut self, shard_id: u64, public_key: sr25519::Public) {
		let shard = self.shards.get_mut(&shard_id).unwrap();
		while let Some(action) = shard.tss.next_action() {
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
					debug!(target: TW_LOG, "Updating tss public key");
					self.timeouts.remove(&(shard_id, None));
					crate::inherents::update_shared_group_key(shard_id, tss_public_key.to_bytes());
				},
				TssAction::Tss(tss_signature, hash) => {
					debug!(target: TW_LOG, "Storing tss signature");
					self.timeouts.remove(&(shard_id, Some(hash)));
					if shard.is_collector {
						let tss_signature = tss_signature.to_bytes();
						let at = self.backend.blockchain().last_finalized().unwrap();
						let signature = self
							.kv
							.sr25519_sign(KEY_TYPE, &public_key, &tss_signature)
							.unwrap()
							.unwrap();
						let _ = self
							.runtime
							.runtime_api()
							.store_signature(
								at,
								public_key.into(),
								signature.into(),
								tss_signature,
								// TODO: set task id
								0u128.into(),
							)
							.unwrap();
					}
				},
				TssAction::Report(offender, hash) => {
					self.timeouts.remove(&(shard_id, hash));
					let Some(proof) = self.kv.sr25519_sign(KEY_TYPE, &public_key, &offender).unwrap() else {
						error!(
							target: TW_LOG,
							"Failed to create proof for offence report submission"
						);
						return;
					};
					let at = self.backend.blockchain().last_finalized().unwrap();
					if let Err(e) = self.runtime.runtime_api().report_misbehavior(
						at,
						shard_id,
						offender.into(),
						public_key.into(),
						proof.into(),
					) {
						error!(
							target: TW_LOG,
							"Offence report runtime API submission failed with reason: {}",
							e.to_string()
						);
					}
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
				notification = self.finality_notifications.next().fuse() => {
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
					self.on_finality(notification, public_key);
				},
				new_sig = self.sign_data_receiver.next().fuse() => {
					let Some((shard_id, data)) = new_sig else {
						continue;
					};
					let Some(public_key) = self.public_key() else {
						continue;
					};
					let Some(shard) = self.shards.get_mut(&shard_id) else {
						continue;
					};
					shard.tss.sign(data.to_vec());
					self.poll_actions(shard_id, public_key);
				},
				gossip_data = self.gossip_data_receiver.next().fuse() => {
					let Some(bytes) = gossip_data else{
						continue;
					};
					log::info!("got tx data for verifying, sending to network",);
					self.gossip_engine.gossip_message(topic::<B>(), bytes, false);
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
						debug!(target: TW_LOG, "received gossip message");
						let shard = self.shards.entry(shard_id).or_insert_with(|| Shard::new(public_key));
						shard.tss.on_message(sender, payload);
						self.poll_actions(shard_id, public_key);

					} else if let Ok(data) = EthTxValidation::decode(&mut &notification.message[..]) {
						debug!(target: TW_LOG, "received gossip message for ethereum transaction validation");
						self.tx_data_sender.clone().try_send(data.encode()).unwrap_or_else(|e| {
							warn!(target: TW_LOG, "Failed to send tx data: {}", e);
						});

					}else{
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
						let shard = self.shards.get_mut(&shard_id);
						if let (Some(shard), Some(timeout)) = (shard, timeout) {
							shard.tss.on_timeout(timeout.timeout);
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
