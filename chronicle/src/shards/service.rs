use super::tss::{Tss, TssAction, VerifiableSecretSharingCommitment};
use crate::admin::AdminMsg;
use crate::network::{Message, Network, PeerId, TssMessage};
use crate::runtime::Runtime;
use crate::tasks::{TaskExecutor, TaskParams};
use crate::TW_LOG;
use anyhow::Result;
use futures::future::join_all;
use futures::SinkExt;
use futures::{
	channel::{mpsc, oneshot},
	future::poll_fn,
	stream::FuturesUnordered,
	Future, FutureExt, Stream, StreamExt,
};
use polkadot_sdk::sp_runtime::BoundedVec;
use std::sync::Arc;
use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	path::PathBuf,
	pin::Pin,
	task::Poll,
};
use time_primitives::{
	BlockHash, BlockNumber, Commitment, ShardId, ShardStatus, TaskId, TssSignature,
	TssSigningRequest,
};
use tracing::{event, span, Level, Span};

pub struct TimeWorkerParams<Tx, Rx> {
	pub substrate: Arc<dyn Runtime>,
	pub task_params: TaskParams,
	pub network: Tx,
	pub tss_request: mpsc::Receiver<TssSigningRequest>,
	pub net_request: Rx,
	pub tss_keyshare_cache: PathBuf,
	pub admin_request: mpsc::Sender<AdminMsg>,
}

pub struct TimeWorker<Tx, Rx> {
	substrate: Arc<dyn Runtime>,
	network: Tx,
	tss_request: mpsc::Receiver<TssSigningRequest>,
	net_request: Rx,
	block_height: u64,
	task_params: TaskParams,
	tss_states: HashMap<ShardId, Tss>,
	executor_states: HashMap<ShardId, TaskExecutor>,
	messages: BTreeMap<BlockNumber, Vec<(ShardId, PeerId, TssMessage)>>,
	requests: BTreeMap<BlockNumber, Vec<(ShardId, TaskId, Vec<u8>)>>,
	channels: HashMap<TaskId, oneshot::Sender<([u8; 32], TssSignature)>>,
	#[allow(clippy::type_complexity)]
	outgoing_requests: FuturesUnordered<
		Pin<Box<dyn Future<Output = (ShardId, PeerId, Result<()>)> + Send + 'static>>,
	>,
	tss_keyshare_cache: PathBuf,
	admin_request: mpsc::Sender<AdminMsg>,
}

impl<Tx, Rx> TimeWorker<Tx, Rx>
where
	Tx: Network + Clone,
	Rx: Stream<Item = (PeerId, Message)> + Send + Unpin,
{
	pub fn new(worker_params: TimeWorkerParams<Tx, Rx>) -> Self {
		let TimeWorkerParams {
			substrate,
			task_params,
			network,
			tss_request,
			net_request,
			tss_keyshare_cache,
			admin_request,
		} = worker_params;
		Self {
			substrate,
			task_params,
			network,
			tss_request,
			net_request,
			block_height: 0,
			tss_states: Default::default(),
			executor_states: Default::default(),
			messages: Default::default(),
			requests: Default::default(),
			channels: Default::default(),
			outgoing_requests: Default::default(),
			tss_keyshare_cache,
			admin_request,
		}
	}

	async fn on_finality(
		&mut self,
		span: &Span,
		block_hash: BlockHash,
		block: BlockNumber,
	) -> Result<()> {
		let span = span!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			"on_finality",
			block = block,
			block_hash = block_hash.to_string(),
		);
		let account_id = self.substrate.account_id();
		let shards = self.substrate.get_shards(account_id).await?;
		self.tss_states.retain(|shard_id, _| shards.contains(shard_id));
		self.executor_states.retain(|shard_id, _| shards.contains(shard_id));
		for shard_id in shards.iter().copied() {
			if self.tss_states.contains_key(&shard_id) {
				continue;
			}
			let span = span!(target: TW_LOG, parent: &span, Level::DEBUG, "join shard", shard_id);
			let members = self.substrate.get_shard_members(shard_id).await?;
			let threshold = self.substrate.get_shard_threshold(shard_id).await?;
			let futures: Vec<_> = members
				.into_iter()
				.map(|(account, _)| {
					let substrate = self.substrate.clone();
					async move {
						match substrate.get_member_peer_id(&account).await {
							Ok(Some(peer_id)) => Some(peer_id),
							Ok(None) | Err(_) => None,
						}
					}
				})
				.collect();
			let members =
				join_all(futures).await.into_iter().flatten().collect::<BTreeSet<PeerId>>();

			let commitment =
				if let Some(commitment) = self.substrate.get_shard_commitment(shard_id).await? {
					let commitment =
						VerifiableSecretSharingCommitment::deserialize(commitment.0.to_vec())?;
					Some(commitment)
				} else {
					None
				};
			self.tss_states.insert(
				shard_id,
				Tss::new(
					self.network.peer_id(),
					members,
					threshold,
					commitment,
					&self.tss_keyshare_cache,
					&span,
				)?,
			);
			if let Err(e) = self.admin_request.send(AdminMsg::JoinedShard(shard_id)).await {
				event!(target: TW_LOG, parent: &span, Level::ERROR, "admin request failed: {:?}", e);
			};
			self.poll_actions(&span, shard_id, block).await;
		}
		for shard_id in shards.iter().copied() {
			let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				continue;
			};
			if tss.committed() {
				continue;
			}
			if self.substrate.get_shard_status(shard_id).await? != ShardStatus::Committed {
				continue;
			}
			event!(target: TW_LOG, parent: &span, Level::DEBUG, shard_id, "committing");
			let commitment = self.substrate.get_shard_commitment(shard_id).await?.unwrap();
			let commitment = VerifiableSecretSharingCommitment::deserialize(commitment.0.to_vec())?;
			tss.on_commit(commitment);
			self.poll_actions(&span, shard_id, block).await;
		}
		while let Some(n) = self.requests.keys().copied().next() {
			if n > block {
				break;
			}
			for (shard_id, task_id, data) in self.requests.remove(&n).unwrap() {
				let span = span!(
					target: TW_LOG,
					parent: &span,
					Level::DEBUG,
					"received signing request from task executor",
					shard_id,
					task_id,
				);
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
					event!(
						target: TW_LOG,
						parent: &span,
						Level::ERROR,
						shard_id,
						task_id,
						"trying to run task on unknown shard, dropping channel",
					);
					self.channels.remove(&task_id);
					continue;
				};
				tss.on_sign(task_id, data.to_vec());
				self.poll_actions(&span, shard_id, block).await;
			}
		}
		for shard_id in shards {
			if self.substrate.get_shard_status(shard_id).await? != ShardStatus::Online {
				continue;
			}
			let executor = self
				.executor_states
				.entry(shard_id)
				.or_insert(TaskExecutor::new(self.task_params.clone()));
			let span = span!(
				target: TW_LOG,
				parent: &span,
				Level::DEBUG,
				"running task executor",
				shard_id,
			);
			let (start_sessions, complete_sessions, failed_tasks) =
				match executor.process_tasks(block, shard_id, self.block_height, &span).await {
					Ok((start_sessions, complete_sessions, failed_tasks)) => {
						(start_sessions, complete_sessions, failed_tasks)
					},
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
			if let Err(e) = self
				.admin_request
				.send(AdminMsg::FailedTasks(start_sessions.len() as u64, failed_tasks))
				.await
			{
				event!(target: TW_LOG, parent: &span, Level::ERROR, shard_id, "Admin request failed: {:?}", e);
			}

			let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				continue;
			};
			for session in complete_sessions {
				tss.on_complete(session);
			}
			for session in start_sessions {
				tss.on_start(session);
			}
		}
		while let Some(n) = self.messages.keys().copied().next() {
			if n > block {
				break;
			}
			for (shard_id, peer_id, msg) in self.messages.remove(&n).unwrap() {
				let span = span!(target: TW_LOG, parent: &span, Level::DEBUG, "messages", shard_id);
				let Some(tss) = self.tss_states.get_mut(&shard_id) else {
					event!(
						target: TW_LOG,
						parent: &span,
						Level::INFO,
						shard_id,
						"dropping message {} from {:?}",
						msg,
						peer_id,
					);
					continue;
				};
				tss.on_message(peer_id, msg)?;
				self.poll_actions(&span, shard_id, n).await;
			}
		}
		Ok(())
	}

	async fn poll_actions(&mut self, span: &Span, shard_id: ShardId, block: BlockNumber) {
		while let Some(action) = self
			.tss_states
			.get_mut(&shard_id)
			.unwrap()
			.next_action(&self.tss_keyshare_cache)
		{
			match action {
				TssAction::Send(msgs) => {
					for (peer, payload) in msgs {
						let msg = Message {
							shard_id,
							block: if payload.is_response() { 0 } else { block },
							payload,
						};
						self.send_message(span, peer, msg);
					}
				},
				TssAction::Commit(commitment, proof_of_knowledge) => {
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						"commit",
					);
					self.substrate
						.submit_commitment(
							shard_id,
							Commitment(BoundedVec::truncate_from(commitment.serialize())),
							proof_of_knowledge.serialize(),
						)
						.await
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
					self.substrate.submit_online(shard_id).await.unwrap();
				},
				TssAction::Signature(task_id, hash, tss_signature) => {
					let tss_signature = tss_signature.to_bytes();
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						task_id,
						"signature {:?}",
						tss_signature,
					);
					if let Some(tx) = self.channels.remove(&task_id) {
						tx.send((hash, tss_signature)).ok();
					}
				},
			}
		}
	}

	fn send_message(&mut self, span: &Span, peer_id: PeerId, message: Message) {
		event!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			shard_id = message.shard_id,
			"tx {} to {:?}",
			message.payload,
			peer_id
		);
		let endpoint = self.network.clone();
		self.outgoing_requests.push(Box::pin(async move {
			let shard_id = message.shard_id;
			let result = endpoint.send(peer_id, message).await;
			(shard_id, peer_id, result)
		}));
	}

	pub async fn run(mut self, span: &Span) {
		event!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			"starting tss",
		);
		let heartbeat_period = self.substrate.get_heartbeat_timeout().await.unwrap();

		// add a future that never resolves to keep outgoing requests alive
		self.outgoing_requests.push(Box::pin(poll_fn(|_| Poll::Pending)));

		let task_params = self.task_params.clone();
		let mut block_stream = task_params.block_stream().fuse();
		let mut block_notifications = self.substrate.block_notification_stream();
		let mut finality_notifications = self.substrate.finality_notification_stream();
		event!(target: TW_LOG, parent: span, Level::INFO, "Started chronicle loop");
		let mut send_heartbeat = true;
		loop {
			futures::select! {
				notification = block_notifications.next().fuse() => {
					let Some((_block_hash, block)) = notification else {
						event!(
							target: TW_LOG,
							parent: span,
							Level::DEBUG,
							"no new block notifications"
						);
						continue;
					};
					if block % heartbeat_period == 0 {
						if send_heartbeat {
							event!(
								target: TW_LOG,
								parent: span,
								Level::ERROR,
								"missed heartbeat period",
							);
						}
						send_heartbeat = true;
					}
					if send_heartbeat {
						event!(
							target: TW_LOG,
							parent: span,
							Level::INFO,
							"submitting heartbeat",
						);
						match self.substrate.submit_heartbeat().await {
							Ok(()) => {
								send_heartbeat = false;
								event!(target: TW_LOG, parent: span, Level::INFO, "submitted heartbeat");
							}
							Err(e) => {
								event!(
									target: TW_LOG,
									parent: span,
									Level::INFO,
									"Error submitting heartbeat: {:?}",
									e
								);
							}
						}
					}
				},
				notification = finality_notifications.next().fuse() => {
					let Some((block_hash, block)) = notification else {
						event!(
							target: TW_LOG,
							parent: span,
							Level::DEBUG,
							"no new finality notifications"
						);
						continue;
					};
					if let Err(e) = self.on_finality(span, block_hash, block).await {
						event!(
							target: TW_LOG,
							parent: span,
							Level::ERROR,
							"Error running on_finality {:?}",
							e
						);
					}
				},
				tss_request = self.tss_request.next().fuse() => {
					let Some(TssSigningRequest { task_id, shard_id, data, tx, block }) = tss_request else {
						continue;
					};
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						task_id,
						block,
						"received signing request",
					);
					self.requests.entry(block).or_default().push((shard_id, task_id, data));
					self.channels.insert(task_id, tx);
				},
				msg = self.net_request.next().fuse() => {
					let Some((peer, Message { shard_id, block, payload })) = msg else {
						continue;
					};
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						block,
						"rx {} from {:?}",
						payload,
						peer,
					);
					self.messages.entry(block).or_default().push((shard_id, peer, payload));
				},
				outgoing_request = self.outgoing_requests.next().fuse() => {
					let Some((shard_id, peer, result)) = outgoing_request else {
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
							parent: span,
							Level::INFO,
							shard_id,
							"tx to {:?} network error {:?}",
							peer,
							error,
						);
					}
				}
				data = block_stream.next() => {
					if let Some(index) = data {
						self.block_height = index;
						if let Err(e) = self.admin_request.send(AdminMsg::TargetBlockReceived).await {
							event!(
								target: TW_LOG,
								parent: span,
								Level::ERROR,
								"Admin request error: {e:?}",
							);
						};
					}
				}
			}
		}
	}
}
