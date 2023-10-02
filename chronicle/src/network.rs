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
	collections::{BTreeMap, BTreeSet, HashMap},
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
	task::Poll,
};
use time_primitives::{
	BlockTimeApi, MembersApi, PublicKey, ShardId, ShardStatus, ShardsApi, SubmitMembers,
	SubmitShards, TaskExecutor, TssId, TssSignature, TssSigningRequest,
};
use tokio::time::{interval_at, sleep, Duration, Instant};

use tracing::{event, span, Level, Span};
use tss::{SigningKey, TssAction, TssMessage, VerifiableSecretSharingCommitment, VerifyingKey};

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
	block_height: u64,
	tss_request: mpsc::Receiver<TssSigningRequest>,
	protocol_request: async_channel::Receiver<IncomingRequest>,
	executor_states: HashMap<ShardId, T>,
	tss_states: HashMap<ShardId, Tss>,
	messages: BTreeMap<u64, Vec<(ShardId, PeerId, TssMessage<TssId>)>>,
	requests: BTreeMap<u64, Vec<(ShardId, TssId, Vec<u8>)>>,
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

impl<B, C, R, N, T, TxSub> TimeWorker<B, C, R, N, T, TxSub>
where
	B: Block + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + BlockTimeApi<B>,
	N: NetworkRequest,
	T: TaskExecutor<B> + Clone,
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

	fn on_finality(&mut self, span: &Span, block: <B as Block>::Hash, block_number: u64) {
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
			.runtime
			.runtime_api()
			.get_shards(block, &self.public_key.clone().into_account())
			.unwrap();
		self.tss_states.retain(|shard_id, _| shards.contains(shard_id));
		self.executor_states.retain(|shard_id, _| shards.contains(shard_id));
		for shard_id in shards.iter().copied() {
			if self.tss_states.get(&shard_id).is_some() {
				continue;
			}
			let api = self.runtime.runtime_api();
			let members = api.get_shard_members(block, shard_id).unwrap();
			event!(
				target: TW_LOG,
				parent: &span,
				Level::DEBUG,
				shard_id,
				"joining shard",
			);
			let threshold = api.get_shard_threshold(block, shard_id).unwrap();
			let members = members
				.into_iter()
				.map(|account| {
					to_peer_id(api.get_member_peer_id(block, &account).unwrap().unwrap())
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
			if self.runtime.runtime_api().get_shard_status(block, shard_id).unwrap()
				!= ShardStatus::Committed
			{
				continue;
			}
			let commitment =
				self.runtime.runtime_api().get_shard_commitment(block, shard_id).unwrap();
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
			if self.runtime.runtime_api().get_shard_status(block, shard_id).unwrap()
				!= ShardStatus::Online
			{
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
				match executor.process_tasks(block, self.block_height, block_number, shard_id) {
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

	fn poll_actions(&mut self, span: &Span, shard_id: ShardId, block_number: u64) {
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
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						"public key {:?}",
						public_key,
					);
					self.tx_submitter
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
		let mut cloned_executor = self.task_executor.clone();
		let mut block_stream = cloned_executor.poll_block_height().await.fuse();

		event!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			"starting tss",
		);
		while let Err(e) = self
			.tx_submitter
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
						event!(
							target: TW_LOG,
							parent: span,
							Level::DEBUG,
							"no new finality notifications"
						);
						continue;
					};
					let block_hash = notification.header.hash();
					let block_number = notification.header.number().to_string().parse().unwrap();
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
					if let Err(e) = self.tx_submitter.submit_heartbeat(self.public_key.clone()).unwrap(){
							event!(
							target: TW_LOG,
							parent: span,
							Level::DEBUG,
							"Error submitting heartbeat {:?}",e
						);
					};
				}
				data = block_stream.next() => {
					if let Some(index) = data {
						self.block_height = index;
					}
				}
			}
		}
	}
}
