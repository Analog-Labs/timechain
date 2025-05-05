use crate::runtime::Runtime;
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
use polkadot_sdk::sp_runtime::BoundedVec;
use scale_codec::Encode;
use std::collections::BTreeMap;
use std::sync::Arc;
use time_primitives::{
	Address32, BlockHash, BlockNumber, ErrorMsg, GmpEvent, GmpEvents, GmpParams, IConnector,
	NetworkId, ShardId, Task, TaskId, TaskResult, TssSignature, TssSigningRequest, MAX_GMP_EVENTS,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{event, span, Instrument, Level, Span};

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

	async fn finalized_block(&self) -> Result<u64> {
		self.connector.finalized_block().await
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

	async fn submit_events(
		&self,
		block_number: BlockNumber,
		shard_id: ShardId,
		task_id: TaskId,
		events: Vec<GmpEvent>,
		span: &Span,
	) -> Result<()> {
		let span = span!(parent: span, Level::INFO, "submit_events", gmp_events = ?events);
		let payload = time_primitives::encode_gmp_events(task_id, &events);
		let signature = self.tss_sign(block_number, shard_id, task_id, payload, &span).await?;
		let result = TaskResult::ReadGatewayEvents {
			events: GmpEvents(BoundedVec::truncate_from(events)),
			signature,
		};
		event!(parent: span, Level::DEBUG, "submitting task result");
		self.runtime.submit_task_result(task_id, result).await
	}

	#[allow(clippy::too_many_arguments)]
	async fn execute(
		self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		cctp_info: Option<(Vec<Address32>, String)>,
		network_id: NetworkId,
		gateway: Address32,
		shard_id: ShardId,
		task_id: TaskId,
		task: Task,
		span: Span,
	) -> Result<()> {
		let span = span!(
			parent: &span,
			Level::INFO,
			"executing task",
			gmp_task_id = task_id,
			gmp_task = %task,
		);
		event!(parent: &span, Level::DEBUG, "executing task");
		match task {
			Task::ReadGatewayEvents { blocks } => {
				tracing::info!(parent: &span, "Starting ReadGatewayEvents({:?})", &blocks);
				let events = self
					.connector
					.read_events(gateway, blocks, cctp_info)
					.instrument(span.clone())
					.await
					.context("read_events")?;
				tracing::info!(parent: &span, "Completed read {} events", events.len());
				let mut remaining = true;
				for chunk in events.chunks(MAX_GMP_EVENTS as _) {
					remaining = chunk.len() != MAX_GMP_EVENTS as usize;
					self.submit_events(block_number, shard_id, task_id, chunk.to_vec(), &span)
						.await?;
				}
				if remaining {
					self.submit_events(block_number, shard_id, task_id, vec![], &span).await?;
				}
			},
			Task::SubmitGatewayMessage { batch_id } => {
				let span =
					span!(parent: &span, Level::INFO, "submit_batch", gmp_batch_id = batch_id);
				let msg = self
					.runtime
					.get_batch_message(batch_id, block_hash)
					.await?
					.context("invalid task")?;
				let payload = GmpParams::new(network_id, gateway).hash(&msg.hash(batch_id));
				let signature =
					self.tss_sign(block_number, shard_id, task_id, payload, &span).await?;
				let signer = self
					.runtime
					.get_shard_commitment(shard_id, block_hash)
					.await?
					.context("invalid shard")?
					.0[0];
				tracing::info!(parent: &span, "submitting batch");
				if let Err(mut e) =
					self.connector.submit_commands(gateway, batch_id, msg, signer, signature).await
				{
					tracing::error!(parent: &span, "Error while executing batch: {e}");
					e.truncate(time_primitives::MAX_ERROR_LEN as usize - 4);
					let result = TaskResult::SubmitGatewayMessage {
						error: ErrorMsg(BoundedVec::truncate_from(e.encode())),
					};
					tracing::debug!(parent: &span, "submitting task result");
					self.runtime.submit_task_result(task_id, result).await?;
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
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		span: &Span,
	) -> Result<(Vec<TaskId>, Vec<TaskId>, u64)> {
		let network = self.params.network();
		let gateway = self
			.params
			.runtime
			.get_gateway(network, block_hash)
			.await?
			.context("no gateway registered")?;
		let cctp_info = self.params.runtime.get_cctp_info(network, block_hash).await?;
		let mut start_sessions = vec![];
		let tasks = self.params.runtime.get_shard_tasks(shard_id, block_hash).await?;

		let failed_tasks: Arc<Mutex<u64>> = Default::default();
		for task_id in tasks.iter().copied() {
			let cctp_info = cctp_info.clone();
			let total_failed = failed_tasks.clone();
			if self.running_tasks.contains_key(&task_id) {
				continue;
			}
			let task = self
				.params
				.runtime
				.get_task(task_id, block_hash)
				.await?
				.context("invalid task")?;

			let chain_block = self.params.finalized_block().await?;
			let span = tracing::span!(
				parent: span,
				Level::INFO,
				"task",
				gmp_task_id = task_id,
				gmp_task = %task,
				chain_block,
			);

			if chain_block < task.start_block() {
				tracing::debug!(
					parent: &span,
					"task scheduled for future {:?}/{:?}",
					chain_block,
					task.start_block(),
				);
				continue;
			}

			tracing::info!(parent: &span, "task started");

			let exec = self.params.clone();
			let span2 = span.clone();
			let handle = tokio::task::spawn(async move {
				let _enter = span2.enter();
				match exec
					.execute(
						block_hash,
						block_number,
						cctp_info,
						network,
						gateway,
						shard_id,
						task_id,
						task,
						span2.clone(),
					)
					.await
				{
					Ok(()) => {
						tracing::info!(parent: &span, "task completed");
					},
					Err(error) => {
						*total_failed.lock().await += 1;
						tracing::error!(parent: &span, ?error, "task failed");
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
						gmp_task_id = task_id,
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
