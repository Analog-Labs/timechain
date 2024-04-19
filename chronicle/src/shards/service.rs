use super::tss::{Tss, TssAction, VerifiableSecretSharingCommitment};
use crate::network::{Message, Network, PeerId, TssMessage};
use crate::tasks::TaskExecutor;
use crate::TW_LOG;
use anyhow::Result;
use futures::future::join_all;
use futures::{
	channel::{mpsc, oneshot},
	future::poll_fn,
	stream::FuturesUnordered,
	Future, FutureExt, Stream, StreamExt,
};
use std::collections::BTreeSet;
use std::{
	collections::{BTreeMap, HashMap},
	pin::Pin,
	task::Poll,
};
use time_primitives::{
	BlockHash, BlockNumber, Runtime, ShardId, ShardStatus, TssId, TssSignature, TssSigningRequest,
};
use tokio::time::{sleep, Duration};
use tracing::{event, span, Level, Span};

pub struct TimeWorkerParams<S, T, Tx, Rx> {
	pub substrate: S,
	pub task_executor: T,
	pub network: Tx,
	pub tss_request: mpsc::Receiver<TssSigningRequest>,
	pub net_request: Rx,
}

pub struct TimeWorker<S, T, Tx, Rx> {
	substrate: S,
	task_executor: T,
	network: Tx,
	tss_request: mpsc::Receiver<TssSigningRequest>,
	net_request: Rx,
	block_height: u64,
	tss_states: HashMap<ShardId, Tss>,
	executor_states: HashMap<ShardId, T>,
	messages: BTreeMap<BlockNumber, Vec<(ShardId, PeerId, TssMessage)>>,
	requests: BTreeMap<BlockNumber, Vec<(ShardId, TssId, Vec<u8>)>>,
	channels: HashMap<TssId, oneshot::Sender<([u8; 32], TssSignature)>>,
	#[allow(clippy::type_complexity)]
	outgoing_requests: FuturesUnordered<
		Pin<Box<dyn Future<Output = (ShardId, PeerId, Result<()>)> + Send + 'static>>,
	>,
}

impl<S, T, Tx, Rx> TimeWorker<S, T, Tx, Rx>
where
	S: Runtime,
	T: TaskExecutor + Clone,
	Tx: Network + Clone,
	Rx: Stream<Item = (PeerId, Message)> + Send + Unpin,
{
	pub fn new(worker_params: TimeWorkerParams<S, T, Tx, Rx>) -> Self {
		let TimeWorkerParams {
			substrate,
			task_executor,
			network,
			tss_request,
			net_request,
		} = worker_params;
		Self {
			substrate,
			task_executor,
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
		}
	}

	async fn on_finality(
		&mut self,
		span: &Span,
		block: BlockHash,
		block_number: BlockNumber,
	) -> Result<()> {
		let span = span!(
			target: TW_LOG,
			parent: span,
			Level::DEBUG,
			"on_finality",
			block = block.to_string(),
			block_number,
		);
		let account_id = self.substrate.account_id();
		let shards = self.substrate.get_shards(block, account_id).await?;
		self.tss_states.retain(|shard_id, _| shards.contains(shard_id));
		self.executor_states.retain(|shard_id, _| shards.contains(shard_id));
		for shard_id in shards.iter().copied() {
			if self.tss_states.get(&shard_id).is_some() {
				continue;
			}
			let members = self.substrate.get_shard_members(block, shard_id).await?;
			event!(
				target: TW_LOG,
				parent: &span,
				Level::DEBUG,
				shard_id,
				"joining shard",
			);
			let threshold = self.substrate.get_shard_threshold(block, shard_id).await?;
			let futures: Vec<_> = members
				.into_iter()
				.map(|(account, _)| {
					let substrate = self.substrate.clone();
					async move {
						match substrate.get_member_peer_id(block, &account).await {
							Ok(Some(peer_id)) => Some(peer_id),
							Ok(None) | Err(_) => None, // Handles both the None and Error cases
						}
					}
				})
				.collect();
			let members =
				join_all(futures).await.into_iter().flatten().collect::<BTreeSet<PeerId>>();

			self.tss_states
				.insert(shard_id, Tss::new(self.network.peer_id(), members, threshold, None)?);
			self.poll_actions(&span, shard_id, block_number).await;
		}
		for shard_id in shards.iter().copied() {
			let Some(tss) = self.tss_states.get_mut(&shard_id) else {
				continue;
			};
			if tss.committed() {
				continue;
			}
			if self.substrate.get_shard_status(block, shard_id).await? != ShardStatus::Committed {
				continue;
			}
			let commitment = self.substrate.get_shard_commitment(block, shard_id).await?;
			let commitment = VerifiableSecretSharingCommitment::deserialize(commitment)?;
			tss.on_commit(commitment);
			self.poll_actions(&span, shard_id, block_number).await;
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
				self.poll_actions(&span, shard_id, block_number).await;
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
						"dropping message {} from {:?}",
						msg,
						peer_id,
					);
					continue;
				};
				if let Some(payload) = tss.on_message(peer_id, msg)? {
					let msg = Message {
						shard_id,
						block_number: 0,
						payload,
					};
					self.send_message(&span, peer_id, msg);
				}
				self.poll_actions(&span, shard_id, n).await;
			}
		}
		for shard_id in shards {
			if self.substrate.get_shard_status(block, shard_id).await? != ShardStatus::Online {
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
			let complete_sessions = match executor
				.process_tasks(block, block_number, shard_id, self.block_height)
				.await
			{
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
		Ok(())
	}

	async fn poll_actions(&mut self, span: &Span, shard_id: ShardId, block_number: BlockNumber) {
		while let Some(action) = self.tss_states.get_mut(&shard_id).unwrap().next_action() {
			match action {
				TssAction::Send(msgs) => {
					for (peer, payload) in msgs {
						let msg = Message {
							shard_id,
							block_number,
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
							commitment.serialize(),
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
		let min_stake = self.substrate.get_min_stake().await.unwrap();
		while let Err(e) = self
			.substrate
			.submit_register_member(self.task_executor.network(), self.network.peer_id(), min_stake)
			.await
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

		let heartbeat_period = self.substrate.get_heartbeat_timeout().await.unwrap();

		// add a future that never resolves to keep outgoing requests alive
		self.outgoing_requests.push(Box::pin(poll_fn(|_| Poll::Pending)));

		let task_executor = self.task_executor.clone();
		let mut block_stream = task_executor.block_stream().fuse();
		let mut finality_notifications = self.substrate.finality_notification_stream();
		event!(target: TW_LOG, parent: span, Level::INFO, "Started chronicle loop");
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
					if block_number % heartbeat_period == 0 {
						event!(
							target: TW_LOG,
							parent: span,
							Level::INFO,
							"submitting heartbeat",
						);
						if let Err(e) = self.substrate.submit_heartbeat().await {
							event!(
								target: TW_LOG,
								parent: span,
								Level::ERROR,
								"Error submitting heartbeat: {:?}",e
							);
						};
					}
					if let Err(e) = self.on_finality(span, block_hash, block_number).await {
						tracing::error!("Error running on_finality {:?}", e);
					}
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
				msg = self.net_request.next().fuse() => {
					let Some((peer, Message { shard_id, block_number, payload })) = msg else {
						continue;
					};
					event!(
						target: TW_LOG,
						parent: span,
						Level::DEBUG,
						shard_id,
						block_number,
						"rx {} from {:?}",
						payload,
						peer,
					);
					self.messages.entry(block_number).or_default().push((shard_id, peer, payload));
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
							parent: &span,
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
					}
				}
			}
		}
	}
}
