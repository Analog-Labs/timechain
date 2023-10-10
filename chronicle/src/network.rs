#![allow(clippy::type_complexity)]
use crate::substrate::SubstrateClient;
use crate::tasks::TaskExecutor;
use crate::{PROTOCOL_NAME, TW_LOG};
use anyhow::Result;
use futures::{
	channel::{mpsc, oneshot},
	future::poll_fn,
	stream::FuturesUnordered,
	Future, FutureExt, StreamExt,
};
use sc_network::config::{IncomingRequest, OutgoingResponse};
use sc_network::{IfDisconnected, NetworkRequest, PeerId};
use serde::{Deserialize, Serialize};
use sp_runtime::traits::IdentifyAccount;
use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	pin::Pin,
	task::Poll,
};
use time_primitives::{
	BlockHash, BlockNumber, Members, PublicKey, ShardId, ShardStatus, Shards, TssId, TssSignature,
	TssSigningRequest,
};
use tokio::time::{interval_at, sleep, Duration, Instant};
use tracing::{event, span, Level, Span};
use tss::{SigningKey, TssAction, TssMessage, VerifiableSecretSharingCommitment, VerifyingKey};

#[derive(Deserialize, Serialize)]
struct TimeMessage {
	shard_id: ShardId,
	block_number: BlockNumber,
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

enum Tss {
	Enabled(tss::Tss<TssId, PeerId>),
	Disabled(SigningKey, Option<TssAction<TssId, PeerId>>, bool),
}

impl Tss {
	fn new(
		peer_id: PeerId,
		members: BTreeSet<PeerId>,
		threshold: u16,
		commitment: Option<VerifiableSecretSharingCommitment>,
	) -> Self {
		if members.len() == 1 {
			let key = SigningKey::random();
			let public = key.public().to_bytes().unwrap();
			let commitment = VerifiableSecretSharingCommitment::deserialize(vec![public]).unwrap();
			let proof_of_knowledge = tss::construct_proof_of_knowledge(
				peer_id,
				&[*key.to_scalar().as_ref()],
				&commitment,
			)
			.unwrap();
			Tss::Disabled(key, Some(TssAction::Commit(commitment, proof_of_knowledge)), false)
		} else {
			Tss::Enabled(tss::Tss::new(peer_id, members, threshold, commitment))
		}
	}

	fn committed(&self) -> bool {
		match self {
			Self::Enabled(tss) => tss.committed(),
			Self::Disabled(_, _, committed) => *committed,
		}
	}

	fn on_commit(&mut self, commitment: VerifiableSecretSharingCommitment) {
		match self {
			Self::Enabled(tss) => tss.on_commit(commitment),
			Self::Disabled(key, actions, committed) => {
				*actions = Some(TssAction::PublicKey(key.public()));
				*committed = true;
			},
		}
	}

	fn on_sign(&mut self, request_id: TssId, data: Vec<u8>) {
		match self {
			Self::Enabled(tss) => tss.on_sign(request_id, data),
			Self::Disabled(key, actions, _) => {
				let hash = VerifyingKey::message_hash(&data);
				*actions = Some(TssAction::Signature(request_id, hash, key.sign_prehashed(hash)));
			},
		}
	}

	fn on_complete(&mut self, request_id: TssId) {
		match self {
			Self::Enabled(tss) => tss.on_complete(request_id),
			Self::Disabled(_, _, _) => {},
		}
	}

	fn on_message(&mut self, peer_id: PeerId, msg: TssMessage<TssId>) -> Option<TssMessage<TssId>> {
		match self {
			Self::Enabled(tss) => tss.on_message(peer_id, msg),
			Self::Disabled(_, _, _) => None,
		}
	}

	fn next_action(&mut self) -> Option<TssAction<TssId, PeerId>> {
		match self {
			Self::Enabled(tss) => tss.next_action(),
			Self::Disabled(_, action, _) => action.take(),
		}
	}
}

pub struct TimeWorkerParams<S, T, N> {
	pub substrate: S,
	pub task_executor: T,
	pub network: N,
	pub public_key: PublicKey,
	pub peer_id: time_primitives::PeerId,
	pub tss_request: mpsc::Receiver<TssSigningRequest>,
	pub protocol_request: async_channel::Receiver<IncomingRequest>,
}

/// Our structure, which holds refs to everything we need to operate
pub struct TimeWorker<S, T, N> {
	substrate: S,
	task_executor: T,
	network: N,
	public_key: PublicKey,
	peer_id: time_primitives::PeerId,
	block_height: u64,
	tss_request: mpsc::Receiver<TssSigningRequest>,
	protocol_request: async_channel::Receiver<IncomingRequest>,
	executor_states: HashMap<ShardId, T>,
	tss_states: HashMap<ShardId, Tss>,
	messages: BTreeMap<BlockNumber, Vec<(ShardId, PeerId, TssMessage<TssId>)>>,
	requests: BTreeMap<BlockNumber, Vec<(ShardId, TssId, Vec<u8>)>>,
	channels: HashMap<TssId, oneshot::Sender<([u8; 32], TssSignature)>>,
	outgoing_requests: FuturesUnordered<
		Pin<
			Box<
				dyn Future<Output = (ShardId, PeerId, TssMessage<TssId>, Result<()>)>
					+ Send
					+ Sync
					+ 'static,
			>,
		>,
	>,
}

impl<S, T, N> TimeWorker<S, T, N>
where
	S: SubstrateClient + Shards + Members,
	T: TaskExecutor + Clone,
	N: NetworkRequest,
{
	pub fn new(worker_params: TimeWorkerParams<S, T, N>) -> Self {
		let TimeWorkerParams {
			substrate,
			task_executor,
			network,
			public_key,
			peer_id,
			tss_request,
			protocol_request,
		} = worker_params;
		Self {
			substrate,
			task_executor,
			network,
			public_key,
			peer_id,
			block_height: 0,
			tss_request,
			protocol_request,
			executor_states: Default::default(),
			tss_states: Default::default(),
			messages: Default::default(),
			requests: Default::default(),
			channels: Default::default(),
			outgoing_requests: Default::default(),
		}
	}

	fn on_finality(&mut self, span: &Span, block: BlockHash, block_number: BlockNumber) {
		let local_peer_id = to_peer_id(self.peer_id);
		let span = span!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			"on_finality",
			block = block.to_string(),
			block_number,
		);
		let shards = self
			.substrate
			.get_shards(block, &self.public_key.clone().into_account())
			.unwrap();
		self.tss_states.retain(|shard_id, _| shards.contains(shard_id));
		self.executor_states.retain(|shard_id, _| shards.contains(shard_id));
		for shard_id in shards.iter().copied() {
			if self.tss_states.get(&shard_id).is_some() {
				continue;
			}
			let members = self.substrate.get_shard_members(block, shard_id).unwrap();
			event!(
				target: TW_LOG,
				parent: &span,
				Level::DEBUG,
				shard_id,
				"joining shard",
			);
			let threshold = self.substrate.get_shard_threshold(block, shard_id).unwrap();
			let members = members
				.into_iter()
				.map(|account| {
					to_peer_id(self.substrate.get_member_peer_id(block, &account).unwrap().unwrap())
				})
				.collect();
			self.tss_states
				.insert(shard_id, Tss::new(local_peer_id, members, threshold, None));
			self.poll_actions(&span, shard_id, block_number);
		}
		for shard_id in shards.iter().copied() {
			let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				continue;
			};
			if tss.committed() {
				continue;
			}
			if self.substrate.get_shard_status(block, shard_id).unwrap() != ShardStatus::Committed {
				continue;
			}
			let commitment = self.substrate.get_shard_commitment(block, shard_id).unwrap();
			let commitment = VerifiableSecretSharingCommitment::deserialize(commitment).unwrap();
			tss.on_commit(commitment);
			self.poll_actions(&span, shard_id, block_number);
		}
		while let Some(n) = self.requests.keys().copied().next() {
			if n > block_number {
				break;
			}
			for (shard_id, request_id, data) in self.requests.remove(&n).unwrap() {
				event!(
					target: TW_LOG,
					parent: &span,
					Level::DEBUG,
					shard_id,
					request_id = format!("{:?}", request_id),
					"received signing request from task executor",
				);
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
					event!(
						target: TW_LOG,
						parent: &span,
						Level::ERROR,
						shard_id,
						request_id = format!("{:?}", request_id),
						"trying to run task on unknown shard, dropping channel",
					);
					self.channels.remove(&request_id);
					continue;
				};
				tss.on_sign(request_id, data.to_vec());
				self.poll_actions(&span, shard_id, block_number);
			}
		}
		while let Some(n) = self.messages.keys().copied().next() {
			if n > block_number {
				break;
			}
			for (shard_id, peer_id, msg) in self.messages.remove(&n).unwrap() {
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
					event!(
						target: TW_LOG,
						parent: &span,
						Level::INFO,
						shard_id,
						"dropping message {} from {}",
						msg,
						peer_id,
					);
					continue;
				};
				if let Some(payload) = tss.on_message(peer_id, msg) {
					let msg = TimeMessage {
						shard_id,
						block_number: 0,
						payload,
					};
					self.send_message(&span, peer_id, msg);
				}
				self.poll_actions(&span, shard_id, n);
			}
		}
		for shard_id in shards {
			if self.substrate.get_shard_status(block, shard_id).unwrap() != ShardStatus::Online {
				continue;
			}
			let executor =
				self.executor_states.entry(shard_id).or_insert(self.task_executor.clone());
			event!(
				target: TW_LOG,
				parent: &span,
				Level::DEBUG,
				shard_id,
				"running task executor"
			);
			let complete_sessions =
				match executor.process_tasks(block, block_number, shard_id, self.block_height) {
					Ok(complete_sessions) => complete_sessions,
					Err(error) => {
						event!(
							target: TW_LOG,
							parent: &span,
							Level::INFO,
							shard_id,
							"failed to start tasks: {:?}",
							error,
						);
						continue;
					},
				};
			let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				continue;
			};
			for session in complete_sessions {
				tss.on_complete(session);
			}
		}
	}

	fn poll_actions(&mut self, span: &Span, shard_id: ShardId, block_number: BlockNumber) {
		while let Some(action) = self.tss_states.get_mut(&shard_id).unwrap().next_action() {
			match action {
				TssAction::Send(msgs) => {
					for (peer, payload) in msgs {
						let msg = TimeMessage {
							shard_id,
							block_number,
							payload,
						};
						self.send_message(span, peer, msg);
					}
				},
				TssAction::Commit(commitment, proof_of_knowledge) => {
					self.substrate
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
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						"public key {:?}",
						public_key,
					);
					self.substrate
						.submit_online(shard_id, self.public_key.clone())
						.unwrap()
						.unwrap();
				},
				TssAction::Signature(request_id, hash, tss_signature) => {
					let tss_signature = tss_signature.to_bytes();
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						request_id = format!("{:?}", request_id),
						"signature {:?}",
						tss_signature,
					);
					if let Some(tx) = self.channels.remove(&request_id) {
						tx.send((hash, tss_signature)).ok();
					}
				},
			}
		}
	}

	fn send_message(&mut self, span: &Span, peer_id: PeerId, message: TimeMessage) {
		event!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			shard_id = message.shard_id,
			"tx {} to {}",
			message.payload,
			peer_id
		);
		let bytes = message.encode();
		let (tx, rx) = oneshot::channel();
		self.network.start_request(
			peer_id,
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
			(message.shard_id, peer_id, message.payload, result)
		}));
	}

	pub async fn run(mut self, span: &Span) {
		event!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			"starting tss",
		);
		while let Err(e) = self
			.substrate
			.submit_register_member(
				self.task_executor.network(),
				self.public_key.clone(),
				self.peer_id,
			)
			.unwrap()
		{
			event!(
				target: TW_LOG,
				parent: span,
				Level::ERROR,
				"Error while submitting member {:?}, retrying again in 10 secs",
				e
			);
			sleep(Duration::from_secs(10)).await;
		}
		event!(
			target: TW_LOG,
			parent: span,
			Level::INFO,
			"Registered Member successfully",
		);

		let min_block_time = self.substrate.get_block_time_in_ms().unwrap();
		let heartbeat_time = (self.substrate.get_heartbeat_timeout().unwrap() / 2) * min_block_time;
		let heartbeat_duration = Duration::from_millis(heartbeat_time);
		let mut heartbeat_tick =
			interval_at(Instant::now() + heartbeat_duration, heartbeat_duration);

		// add a future that never resolves to keep outgoing requests alive
		self.outgoing_requests.push(Box::pin(poll_fn(|_| Poll::Pending)));

		let task_executor = self.task_executor.clone();
		let mut block_stream = task_executor.block_stream();
		let mut finality_notifications = self.substrate.finality_notification_stream();
		loop {
			futures::select! {
				notification = finality_notifications.next().fuse() => {
					let Some((block_hash, block_number)) = notification else {
						event!(
							target: TW_LOG,
							parent: span,
							Level::DEBUG,
							"no new finality notifications"
						);
						continue;
					};
					self.on_finality(span, block_hash, block_number);
				},
				tss_request = self.tss_request.next().fuse() => {
					let Some(TssSigningRequest { request_id, shard_id, data, tx, block_number }) = tss_request else {
						continue;
					};
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						request_id = format!("{:?}", request_id),
						block_number,
						"received signing request",
					);
					self.requests.entry(block_number).or_default().push((shard_id, request_id, data));
					self.channels.insert(request_id, tx);
				},
				protocol_request = self.protocol_request.next().fuse() => {
					let Some(IncomingRequest { peer, payload, pending_response }) = protocol_request else {
						continue;
					};
					let span = span!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						"received network request",
					);
					// Don't try to do anything other than reply immediately as substrate will close the substream.
					let _ = pending_response.send(OutgoingResponse {
						result: Ok(vec![]),
						reputation_changes: vec![],
						sent_feedback: None,
					});
					if let Ok(TimeMessage { shard_id, block_number, payload }) = TimeMessage::decode(&payload) {
						event!(
							target: TW_LOG,
							parent: &span,
							Level::DEBUG,
							shard_id,
							block_number,
							"rx {} from {}",
							payload,
							peer,
						);
						self.messages.entry(block_number).or_default().push((shard_id, peer, payload));
					} else {
						event!(
							target: TW_LOG,
							parent: &span,
							Level::DEBUG,
							"received invalid message",
						);
					}
				},
				outgoing_request = self.outgoing_requests.next().fuse() => {
					let Some((shard_id, peer, request, result)) = outgoing_request else {
						continue;
					};
					let span = span!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						"received response",
						shard_id,
					);
					if let Err(error) = result {
						event!(
							target: TW_LOG,
							parent: &span,
							Level::INFO,
							shard_id,
							"tx {} to {} network error {:?}",
							request,
							peer,
							error,
						);
					}
				}
				_ = heartbeat_tick.tick().fuse() => {
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						"submitting heartbeat",
					);
					if let Err(e) = self.substrate.submit_heartbeat(self.public_key.clone()).unwrap(){
							event!(
							target: TW_LOG,
							parent: span,
							Level::DEBUG,
							"Error submitting heartbeat {:?}",e
						);
					};
				}
				data = block_stream.next().fuse() => {
					if let Some(index) = data {
						self.block_height = index;
					}
				}
			}
		}
	}
}
