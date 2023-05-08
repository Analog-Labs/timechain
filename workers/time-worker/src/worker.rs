#![allow(clippy::type_complexity)]
use crate::{
	communication::validator::{topic, GossipValidator},
	kv::TimeKeyvault,
	Client, WorkerParams, TW_LOG,
};
use futures::{channel::mpsc, FutureExt, StreamExt};
use log::{debug, error, warn};
use sc_client_api::{Backend, FinalityNotification, FinalityNotifications};
use sc_network_gossip::GossipEngine;
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_core::{sr25519, Pair};
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
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
use time_primitives::{TimeApi, KEY_TYPE};
use tokio::time::Sleep;
use tss::{Timeout, Tss, TssAction, TssMessage};

fn sign(
	store: &SyncCryptoStorePtr,
	public: sr25519::Public,
	message: &[u8],
) -> Option<sr25519::Signature> {
	let sig = SyncCryptoStore::sign_with(&**store, KEY_TYPE, &public.into(), message)
		.ok()??
		.try_into()
		.ok()?;
	Some(sr25519::Signature::from_raw(sig))
}

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: u64,
	sender: sr25519::Public,
	payload: TssMessage,
}

impl TimeMessage {
	fn encode(&self, kv: &TimeKeyvault) -> Vec<u8> {
		let kv = kv.get_store().unwrap();
		let mut bytes = bincode::serialize(self).unwrap();
		let sig = sign(&kv, self.sender, &bytes).unwrap();
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
	timeout: Option<TssTimeout>,
}

impl Shard {
	fn new(public: sr25519::Public) -> Self {
		Self {
			tss: Tss::new(public),
			timeout: None,
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
	kv: TimeKeyvault,
	shards: HashMap<u64, Shard>,
	finality_notifications: FinalityNotifications<B>,
	gossip_engine: GossipEngine<B>,
	_gossip_validator: Arc<GossipValidator<B>>,
	sign_data_receiver: mpsc::Receiver<(u64, [u8; 32])>,
	accountid: PhantomData<A>,
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
			shards: Default::default(),
			accountid,
			timeout: None,
		}
	}

	/// Returns the public key for the worker if one was set.
	fn public_key(&self) -> Option<sr25519::Public> {
		let keys = self.kv.public_keys();
		if keys.is_empty() {
			warn!(target: TW_LOG, "No time key found, please inject one.");
			return None;
		}
		Some(*keys[0].as_ref())
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
					shard.timeout = None;
					crate::inherents::update_shared_group_key(shard_id, tss_public_key.to_bytes());
				},
				TssAction::Tss(tss_signature) => {
					debug!(target: TW_LOG, "Storing tss signature");
					shard.timeout = None;
					let tss_signature = tss_signature.to_bytes();
					let at = self.backend.blockchain().last_finalized().unwrap();
					let signature = self.kv.sign(&public_key.into(), &tss_signature).unwrap();
					self.runtime
						.runtime_api()
						.store_signature(
							at,
							public_key.into(),
							signature,
							tss_signature,
							// TODO: set task id
							0u128.into(),
						)
						.unwrap();
				},
				TssAction::Report(offender) => {
					shard.timeout = None;
					let Some(proof) = self.kv.sign(&public_key.into(), &offender) else {
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
						proof,
					) {
						error!(
							target: TW_LOG,
							"Offence report runtime API submission failed with reason: {}",
							e.to_string()
						);
					}
				},
				TssAction::Timeout(timeout) => {
					let timeout = TssTimeout::new(timeout);
					if self.timeout.is_none() {
						self.timeout = Some(sleep_until(timeout.deadline));
					}
					shard.timeout = Some(timeout);
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
				gossip = gossips.next().fuse() => {
					let Some(notification) = gossip else {
						debug!(target: TW_LOG, "no new gossip");
						continue;
					};
					let Ok(TimeMessage { shard_id, sender, payload }) = TimeMessage::decode(&notification.message) else {
						debug!(target: TW_LOG, "received invalid message");
						continue;
					};
					let Some(public_key) = self.public_key() else {
						continue;
					};
					debug!(target: TW_LOG, "received gossip message");
					let shard = self.shards.entry(shard_id).or_insert_with(|| Shard::new(public_key));
					shard.tss.on_message(sender, payload);
					self.poll_actions(shard_id, public_key);
				},
				_ = timeout.fuse() => {
					let mut next_timeout = None;
					let now = Instant::now();
					for shard in self.shards.values_mut() {
						if let Some(timeout) = shard.timeout.as_ref() {
							if timeout.deadline <= now {
								shard.tss.on_timeout(timeout.timeout);
								shard.timeout = None;
							} else if let Some(deadline) = next_timeout {
								if timeout.deadline < deadline {
									next_timeout = Some(timeout.deadline);
								}
							} else if next_timeout.is_none() {
								next_timeout = Some(timeout.deadline);
							}
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
