#![allow(clippy::type_complexity)]
use crate::{PROTOCOL_NAME, TW_LOG};
use anyhow::Result;
use futures::{
	channel::{mpsc, oneshot},
	FutureExt, StreamExt,
};
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::config::{IncomingRequest, OutgoingResponse};
use sc_network::{IfDisconnected, NetworkRequest, PeerId};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Block, Header, IdentifyAccount};
use std::{
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	sync::Arc,
};
use time_primitives::{
	MembersApi, PublicKey, ShardId, ShardsApi, SubmitMembers, SubmitShards, TaskExecutor, TssId,
	TssRequest, TssSignature,
};
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

pub struct WorkerParams<B: Block, C, R, N, T, TxSub> {
	pub _block: PhantomData<B>,
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub network: N,
	pub task_executor: T,
	pub tx_submitter: TxSub,
	pub public_key: PublicKey,
	pub peer_id: time_primitives::PeerId,
	pub tss_request: mpsc::Receiver<TssRequest>,
	pub protocol_request: async_channel::Receiver<IncomingRequest>,
}

/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<B: Block, C, R, N, T, TxSub> {
	_block: PhantomData<B>,
	client: Arc<C>,
	runtime: Arc<R>,
	network: N,
	task_executor: T,
	tx_submitter: TxSub,
	public_key: PublicKey,
	peer_id: time_primitives::PeerId,
	tss_request: mpsc::Receiver<TssRequest>,
	protocol_request: async_channel::Receiver<IncomingRequest>,
	tss_states: HashMap<ShardId, Tss<TssId, PeerId>>,
	messages: BTreeMap<u64, Vec<(ShardId, PeerId, TssMessage<TssId>)>>,
	requests: BTreeMap<u64, Vec<(ShardId, TssId, Vec<u8>)>>,
	channels: HashMap<TssId, oneshot::Sender<([u8; 32], TssSignature)>>,
}

impl<B, C, R, N, T, TxSub> TimeWorker<B, C, R, N, T, TxSub>
where
	B: Block + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: MembersApi<B> + ShardsApi<B>,
	N: NetworkRequest,
	T: TaskExecutor<B>,
	TxSub: SubmitShards<B> + SubmitMembers<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, C, R, N, T, TxSub>) -> Self {
		let WorkerParams {
			_block,
			client,
			runtime,
			network,
			task_executor,
			tx_submitter,
			public_key,
			peer_id,
			tss_request,
			protocol_request,
		} = worker_params;
		Self {
			_block,
			client,
			runtime,
			network,
			task_executor,
			tx_submitter,
			public_key,
			peer_id,
			tss_request,
			protocol_request,
			tss_states: Default::default(),
			messages: Default::default(),
			requests: Default::default(),
			channels: Default::default(),
		}
	}

	fn on_finality(&mut self, block: <B as Block>::Hash, block_number: u64) {
		let local_peer_id = to_peer_id(self.peer_id);
		log::debug!(target: TW_LOG, "{}: on_finality {}", local_peer_id, block.to_string());
		let shards = self
			.runtime
			.runtime_api()
			.get_shards(block, &self.public_key.clone().into_account())
			.unwrap();
		for shard_id in shards.iter().copied() {
			if self.tss_states.get(&shard_id).is_some() {
				continue;
			}
			let api = self.runtime.runtime_api();
			let members = api.get_shard_members(block, shard_id).unwrap();
			log::debug!(target: TW_LOG, "shard {}: {} joining shard", shard_id, local_peer_id);
			let threshold = api.get_shard_threshold(block, shard_id).unwrap();
			let members = members
				.into_iter()
				.map(|account| {
					to_peer_id(api.get_member_peer_id(block, &account).unwrap().unwrap())
				})
				.collect();
			self.tss_states.insert(shard_id, Tss::new(local_peer_id, members, threshold));
			self.poll_actions(shard_id, block, block_number);
		}
		while let Some(n) = self.requests.keys().copied().next() {
			if n > block_number {
				break;
			}
			for (shard_id, request_id, data) in self.requests.remove(&n).unwrap() {
				log::debug!(target: TW_LOG, "shard {}: req {:?}: sign", shard_id, request_id);
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
					log::error!(target: TW_LOG, "trying to run task on unknown shard {}, dropping channel", shard_id);
					self.channels.remove(&request_id);
					continue;
				};
				tss.sign(request_id, data.to_vec());
				self.poll_actions(shard_id, block, block_number);
			}
		}
		while let Some(n) = self.messages.keys().copied().next() {
			if n > block_number {
				break;
			}
			for (shard_id, peer_id, msg) in self.messages.remove(&n).unwrap() {
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				log::error!(target: TW_LOG, "dropping message {} {} {}", shard_id, peer_id, msg);
				continue;
			};
				tss.on_message(peer_id, msg);
				self.poll_actions(shard_id, block, n);
			}
		}
		for shard_id in shards {
			let task_executor = self.task_executor.clone();
			tokio::task::spawn(async move {
				log::info!(target: TW_LOG, "shard {}: running task executor", shard_id);
				if let Err(err) = task_executor.start_tasks(block, block_number, shard_id).await {
					log::error!(target: TW_LOG, "shard {}: failed to start tasks: {:?}", shard_id, err);
				}
			});
		}
	}

	fn poll_actions(&mut self, shard_id: ShardId, block: <B as Block>::Hash, block_number: u64) {
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
									"shard {}: {} tx {} to {} network error {:?}",
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
					let public_key = tss_public_key.to_bytes().unwrap();
					log::info!(target: TW_LOG, "shard {}: public key {:?}", shard_id, public_key);
					if let Err(e) =
						self.tx_submitter.submit_tss_pub_key(block, shard_id, public_key)
					{
						log::error!("error submitting tss pubkey {:?}", e);
					}
				},
				TssAction::Signature(request_id, hash, tss_signature) => {
					let tss_signature = tss_signature.to_bytes();
					log::debug!(
						target: TW_LOG,
						"shard {}: req {:?}: signature {:?}",
						shard_id,
						request_id,
						tss_signature
					);
					if let Some(tx) = self.channels.remove(&request_id) {
						tx.send((hash, tss_signature)).ok();
					}
				},
			}
		}
	}

	/// Our main worker main process - we act on grandpa finality and gossip messages for interested
	/// topics
	pub(crate) async fn run(&mut self) {
		let block = self.client.info().best_hash;
		if let Err(e) = self.tx_submitter.submit_register_member(
			block,
			self.task_executor.network(),
			self.public_key.clone(),
			self.peer_id,
		) {
			log::error!("error registering member {:?}", e);
		}
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
					log::debug!(target: TW_LOG, "finalized {}", block_number);
					self.on_finality(block_hash, block_number);
				},
				tss_request = self.tss_request.next().fuse() => {
					let Some(TssRequest { request_id, shard_id, data, tx, block_number }) = tss_request else {
						continue;
					};
					self.requests.entry(block_number).or_default().push((shard_id, request_id, data));
					self.channels.insert(request_id, tx);
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
						self.messages.entry(block_number).or_default().push((shard_id, peer, payload));
					} else {
						log::debug!(target: TW_LOG, "received invalid message");
						continue;
					}
				},
			}
		}
	}
}
