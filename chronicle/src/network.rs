#![allow(clippy::type_complexity)]
use crate::{PROTOCOL_NAME, TW_LOG};
use anyhow::Result;
use futures::{
	channel::{mpsc, oneshot},
	future::poll_fn,
	stream::FuturesUnordered,
	Future, FutureExt, StreamExt,
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
	pin::Pin,
	sync::Arc,
	task::Poll,
};
use time_primitives::{
	BlockTimeApi, MembersApi, PublicKey, ShardId, ShardStatus, ShardsApi, SubmitMembers,
	SubmitShards, TaskExecutor, TssId, TssSignature, TssSigningRequest,
};
use tokio::time::{interval_at, Duration, Instant};
use tss::{Tss, TssAction, TssRequest, TssResponse, VerifiableSecretSharingCommitment};

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: ShardId,
	block_number: u64,
	payload: TssRequest<TssId>,
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

pub struct TimeWorkerParams<B: Block, C, R, N, T, TxSub> {
	pub _block: PhantomData<B>,
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub network: N,
	pub task_executor: T,
	pub tx_submitter: TxSub,
	pub public_key: PublicKey,
	pub peer_id: time_primitives::PeerId,
	pub tss_request: mpsc::Receiver<TssSigningRequest>,
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
	tss_request: mpsc::Receiver<TssSigningRequest>,
	protocol_request: async_channel::Receiver<IncomingRequest>,
	tss_states: HashMap<ShardId, Tss<TssId, PeerId>>,
	messages:
		BTreeMap<u64, Vec<(ShardId, PeerId, TssRequest<TssId>, oneshot::Sender<OutgoingResponse>)>>,
	requests: BTreeMap<u64, Vec<(ShardId, TssId, Vec<u8>)>>,
	channels: HashMap<TssId, oneshot::Sender<([u8; 32], TssSignature)>>,
	outgoing_requests: FuturesUnordered<
		Pin<
			Box<
				dyn Future<
						Output = (
							ShardId,
							PeerId,
							TssRequest<TssId>,
							Result<Option<TssResponse<TssId>>>,
						),
					> + Send
					+ Sync
					+ 'static,
			>,
		>,
	>,
}

impl<B, C, R, N, T, TxSub> TimeWorker<B, C, R, N, T, TxSub>
where
	B: Block + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + BlockTimeApi<B>,
	N: NetworkRequest,
	T: TaskExecutor<B>,
	TxSub: SubmitShards + SubmitMembers,
{
	pub fn new(worker_params: TimeWorkerParams<B, C, R, N, T, TxSub>) -> Self {
		let TimeWorkerParams {
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
			outgoing_requests: Default::default(),
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
		self.tss_states.retain(|shard_id, _| shards.contains(shard_id));
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
			self.tss_states
				.insert(shard_id, Tss::new(local_peer_id, members, threshold, None));
			self.poll_actions(shard_id, block_number);
		}
		for shard_id in shards.iter().copied() {
			let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				continue;
			};
			if tss.committed() {
				continue;
			}
			if self.runtime.runtime_api().get_shard_status(block, shard_id).unwrap()
				!= ShardStatus::Committed
			{
				continue;
			}
			let commitment =
				self.runtime.runtime_api().get_shard_commitment(block, shard_id).unwrap();
			let commitment = VerifiableSecretSharingCommitment::deserialize(commitment).unwrap();
			tss.on_commit(commitment);
			self.poll_actions(shard_id, block_number);
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
				tss.on_sign(request_id, data.to_vec());
				self.poll_actions(shard_id, block_number);
			}
		}
		while let Some(n) = self.messages.keys().copied().next() {
			if n > block_number {
				break;
			}
			for (shard_id, peer_id, msg, pending_response) in self.messages.remove(&n).unwrap() {
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
					log::error!(target: TW_LOG, "dropping message {} {} {}", shard_id, peer_id, msg);
					continue;
				};
				let result = tss
					.on_request(peer_id, msg)
					.map(|msg| bincode::serialize(&msg).unwrap())
					.map_err(|error| log::error!(target: TW_LOG, "invalid request: {:?}", error));
				let _ = pending_response.send(OutgoingResponse {
					result,
					reputation_changes: vec![],
					sent_feedback: None,
				});
				self.poll_actions(shard_id, n);
			}
		}
		for shard_id in shards {
			if self.runtime.runtime_api().get_shard_status(block, shard_id).unwrap()
				!= ShardStatus::Online
			{
				continue;
			}
			log::info!(target: TW_LOG, "shard {}: running task executor", shard_id);
			if let Err(err) = self.task_executor.start_tasks(block, block_number, shard_id) {
				log::error!(target: TW_LOG, "shard {}: failed to start tasks: {:?}", shard_id, err);
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
						self.outgoing_requests.push(Box::pin(async move {
							let result = async move {
								let response = rx.await??;
								Ok(bincode::deserialize(&response)?)
							}
							.await;
							(shard_id, peer, msg.payload, result)
						}));
					}
				},
				TssAction::Commit(commitment, proof_of_knowledge) => {
					self.tx_submitter
						.submit_commitment(
							shard_id,
							self.public_key.clone(),
							commitment.serialize(),
							proof_of_knowledge.serialize(),
						)
						.unwrap()
						.unwrap();
				},
				TssAction::PublicKey(tss_public_key) => {
					let public_key = tss_public_key.to_bytes().unwrap();
					log::info!(target: TW_LOG, "shard {}: public key {:?}", shard_id, public_key);
					self.tx_submitter
						.submit_online(shard_id, self.public_key.clone())
						.unwrap()
						.unwrap();
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

	pub async fn run(mut self) {
		log::info!(target: TW_LOG, "starting tss");
		self.tx_submitter
			.submit_register_member(
				self.task_executor.network(),
				self.public_key.clone(),
				self.peer_id,
			)
			.unwrap()
			.unwrap();

		let block = self.client.info().best_hash;
		let min_block_time = self.runtime.runtime_api().get_block_time_in_msec(block).unwrap();
		let heartbeat_time =
			(self.runtime.runtime_api().get_heartbeat_timeout(block).unwrap() / 2) * min_block_time;
		let heartbeat_duration = Duration::from_millis(heartbeat_time);
		let mut heartbeat_tick =
			interval_at(Instant::now() + heartbeat_duration, heartbeat_duration);
		// add a future that never resolves to keep outgoing requests alive
		self.outgoing_requests.push(Box::pin(poll_fn(|_| Poll::Pending)));

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
					log::debug!(target: TW_LOG, "received signing request");
					let Some(TssSigningRequest { request_id, shard_id, data, tx, block_number }) = tss_request else {
						continue;
					};
					self.requests.entry(block_number).or_default().push((shard_id, request_id, data));
					self.channels.insert(request_id, tx);
				},
				protocol_request = self.protocol_request.next().fuse() => {
					log::debug!(target: TW_LOG, "received request");
					let Some(IncomingRequest { peer, payload, pending_response }) = protocol_request else {
						continue;
					};
					if let Ok(TimeMessage { shard_id, block_number, payload }) = TimeMessage::decode(&payload) {
						let local_peer_id = to_peer_id(self.peer_id);
						log::debug!(target: TW_LOG, "shard {}: {} rx {} from {}",
							shard_id, local_peer_id, payload, peer);
						self.messages.entry(block_number).or_default().push((shard_id, peer, payload, pending_response));
					} else {
						log::debug!(target: TW_LOG, "received invalid message");
						let _ = pending_response.send(OutgoingResponse {
							result: Err(()),
							reputation_changes: vec![],
							sent_feedback: None,
						});
					}
				},
				outgoing_request = self.outgoing_requests.next().fuse() => {
					log::debug!(target: TW_LOG, "received response");
					let Some((shard_id, peer, request, result)) = outgoing_request else {
						continue;
					};
					let Some(tss) = self.tss_states.get_mut(&shard_id) else {
						continue;
					};
					match result {
						Ok(response) => {
							tss.on_response(peer, response);
						}
						Err(error) => {
							let local_peer_id = to_peer_id(self.peer_id);
							log::error!(
								target: TW_LOG,
								"shard {}: {} tx {} to {} network error {:?}",
								shard_id,
								local_peer_id,
								request,
								peer,
								error,
							);
						}
					}
				}
				_ = heartbeat_tick.tick().fuse() => {
					log::debug!(target: TW_LOG, "submitting heartbeat");
					self.tx_submitter.submit_heartbeat(self.public_key.clone()).unwrap().unwrap();
				}
				_ = self.task_executor.poll_block_height().fuse() => {}
			}
		}
	}
}
