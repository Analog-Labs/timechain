use crate::runtime::Runtime;
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, Stream};
use polkadot_sdk::sp_runtime::BoundedVec;
use scale_codec::Encode;
use std::sync::Arc;
use std::{collections::BTreeMap, pin::Pin};
use time_primitives::{
	Address, BlockNumber, ErrorMsg, GmpEvents, GmpParams, IConnector, NetworkId, ShardId, Task,
	TaskId, TaskResult, TssSignature, TssSigningRequest,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{event, span, Level, Span};

#[derive(Clone)]
pub struct TaskParams {
	tss: mpsc::Sender<TssSigningRequest>,
	runtime: Arc<dyn Runtime>,
	connector: Arc<dyn IConnector>,
}

impl TaskParams {
	pub fn new(
		runtime: Arc<dyn Runtime>,
		connector: Arc<dyn IConnector>,
		tss: mpsc::Sender<TssSigningRequest>,
	) -> Self {
		Self { runtime, connector, tss }
	}

	pub fn network(&self) -> NetworkId {
		self.connector.network_id()
	}

	pub fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		self.connector.block_stream()
	}

	async fn tss_sign(
		&self,
		block: BlockNumber,
		shard_id: ShardId,
		task_id: TaskId,
		data: Vec<u8>,
		span: &Span,
	) -> Result<TssSignature> {
		tracing::debug!(parent: span, "tss_sign");
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssSigningRequest {
				task_id,
				shard_id,
				block,
				data,
				tx,
			})
			.await?;
		let (_, sig) = rx.await?;
		Ok(sig)
	}

	async fn is_executable(
		&self,
		task_id: TaskId,
		task: &Task,
		target_block_height: u64,
		span: &Span,
	) -> Result<bool> {
		if target_block_height < task.start_block() {
			tracing::debug!(
				parent: span,
				task_id,
				task = task.to_string(),
				target_block_height,
				"task scheduled for future {:?}/{:?}",
				target_block_height,
				task.start_block(),
			);
			return Ok(false);
		}
		if task.needs_signer() {
			let Some(public_key) = self.runtime.get_task_submitter(task_id).await? else {
				tracing::debug!(
					parent: span,
					"no submitter set for task",
				);
				return Ok(false);
			};
			if &public_key != self.runtime.public_key() {
				return Ok(false);
			}
		}
		Ok(true)
	}

	#[allow(clippy::too_many_arguments)]
	async fn execute(
		self,
		block_number: BlockNumber,
		network_id: NetworkId,
		gateway: Address,
		shard_id: ShardId,
		task_id: TaskId,
		task: Task,
		span: Span,
	) -> Result<()> {
		match task {
			Task::ReadGatewayEvents { blocks } => {
				let events =
					self.connector.read_events(gateway, blocks).await.context("read_events")?;
				if events.is_empty() {
					tracing::info!(parent: &span, "No read events to process.");
					return Ok(());
				}
				const MAX_EVENTS: usize = time_primitives::task::MAX_GMP_EVENTS as usize;
				let total_events = events.len();
				tracing::info!(parent: &span, "read {} events", total_events);
				let event_chunks: Vec<Vec<_>> =
					events.chunks(MAX_EVENTS).map(|chunk| chunk.to_vec()).collect();
				let total_chunks = event_chunks.len();
				for (i, chunk) in event_chunks.into_iter().enumerate() {
					let payload = time_primitives::encode_gmp_events(task_id, &chunk);
					let signature =
						self.tss_sign(block_number, shard_id, task_id, payload, &span).await?;
					debug_assert!(
						chunk.len() <= MAX_EVENTS,
						"WARNING: Truncation threw out {} read gateway events!",
						chunk.len() - MAX_EVENTS
					);
					let result = TaskResult::ReadGatewayEvents {
						events: GmpEvents(BoundedVec::truncate_from(chunk)),
						signature,
						remaining: i != total_chunks - 1,
					};
					tracing::debug!(parent: &span, "submitting task result",);
					if let Err(e) = self.runtime.submit_task_result(task_id, result).await {
						tracing::error!(
							parent: &span,
							"error submitting task result: {:?}",
							e
						);
					}
				}
			},
			Task::SubmitGatewayMessage { batch_id } => {
				let msg =
					self.runtime.get_batch_message(batch_id).await?.context("invalid task")?;
				let payload = GmpParams::new(network_id, gateway).hash(&msg.encode(batch_id));
				let signature =
					self.tss_sign(block_number, shard_id, task_id, payload, &span).await?;
				let signer =
					self.runtime.get_shard_commitment(shard_id).await?.context("invalid shard")?.0
						[0];
				if let Err(mut e) =
					self.connector.submit_commands(gateway, batch_id, msg, signer, signature).await
				{
					tracing::error!(parent: &span, batch_id, "Error while executing batch: {e}");
					e.truncate(time_primitives::MAX_ERROR_LEN as usize - 4);
					let result = TaskResult::SubmitGatewayMessage {
						error: ErrorMsg(BoundedVec::truncate_from(e.encode())),
					};
					tracing::debug!(parent: &span, "submitting task result",);
					if let Err(e) = self.runtime.submit_task_result(task_id, result).await {
						tracing::error!(
							parent: &span,
							"error submitting task result: {:?}",
							e
						);
					}
				}
			},
		}
		Ok(())
	}
}

pub struct TaskExecutor {
	params: TaskParams,
	running_tasks: BTreeMap<TaskId, JoinHandle<()>>,
}

impl TaskExecutor {
	pub fn new(params: TaskParams) -> Self {
		Self {
			params,
			running_tasks: Default::default(),
		}
	}

	pub async fn process_tasks(
		&mut self,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
		span: &Span,
	) -> Result<(Vec<TaskId>, Vec<TaskId>, u64)> {
		let network = self.params.network();
		let gateway = self
			.params
			.runtime
			.get_gateway(network)
			.await?
			.context("no gateway registered")?;
		let mut start_sessions = vec![];
		let tasks = self.params.runtime.get_shard_tasks(shard_id).await?;

		let failed_tasks: Arc<Mutex<u64>> = Default::default();
		for task_id in tasks.iter().copied() {
			let total_failed = failed_tasks.clone();
			if self.running_tasks.contains_key(&task_id) {
				continue;
			}
			let task = self.params.runtime.get_task(task_id).await?.context("invalid task")?;
			if !self.params.is_executable(task_id, &task, target_block_height, span).await? {
				continue;
			}

			let span = span!(
				parent: span,
				Level::INFO,
				"task started",
				task_id,
				%task,
				target_block_height,
			);
			let exec = self.params.clone();
			let span2 = span.clone();
			let handle = tokio::task::spawn(async move {
				match exec
					.execute(block_number, network, gateway, shard_id, task_id, task, span2)
					.await
				{
					Ok(()) => {
						tracing::info!(parent: &span, task_id, target_block_height, "task completed");
					},
					Err(error) => {
						*total_failed.lock().await += 1;
						tracing::error!(parent: &span, task_id, target_block_height, ?error, "task failed");
					},
				};
			});
			start_sessions.push(task_id);
			self.running_tasks.insert(task_id, handle);
		}
		let mut completed_sessions = Vec::with_capacity(self.running_tasks.len());
		// remove from running task if task is completed or we dont receive anymore from pallet
		self.running_tasks.retain(|task_id, handle| {
			if tasks.contains(task_id) {
				true
			} else {
				if !handle.is_finished() {
					event!(
						parent: span,
						Level::DEBUG,
						task_id,
						"task aborted",
					);
					handle.abort();
				}
				completed_sessions.push(*task_id);
				false
			}
		});
		let failed_tasks = *failed_tasks.lock().await;
		Ok((start_sessions, completed_sessions, failed_tasks))
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_event_chunking_preserves_all_events() {
		const MAX_EVENTS: usize = 4;

		// Create a sample list of events
		let events: Vec<u32> = (1..=11).collect();

		// Perform chunking
		let event_chunks: Vec<Vec<_>> =
			events.chunks(MAX_EVENTS).map(|chunk| chunk.to_vec()).collect();

		// Flatten the chunks back into a single vector
		let flattened_events: Vec<_> = event_chunks.iter().flatten().copied().collect();

		// Ensure the original and reconstructed lists match
		assert_eq!(events, flattened_events, "Chunking should not lose any events");
	}
}
