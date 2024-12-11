use crate::runtime::Runtime;
use crate::TW_LOG;
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, Stream};
use polkadot_sdk::sp_runtime::BoundedVec;
use scale_codec::Encode;
use std::sync::Arc;
use std::{collections::BTreeMap, pin::Pin};
use time_primitives::{
	Address, BlockHash, BlockNumber, ErrorMsg, GatewayMessage, GatewayOp, GmpEvents, GmpParams,
	IConnector, NetworkId, ShardId, Task, TaskId, TaskResult, TssSignature, TssSigningRequest,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{event, span, Level};

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
		block_number: BlockNumber,
		shard_id: ShardId,
		task_id: TaskId,
		data: Vec<u8>,
	) -> Result<TssSignature> {
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssSigningRequest {
				task_id,
				shard_id,
				block_number,
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
	) -> Result<bool> {
		if target_block_height < task.start_block() {
			tracing::debug!(target: TW_LOG, task_id,
				"task scheduled for future {:?}/{:?}",
				target_block_height,
				task.start_block(),
			);
			return Ok(false);
		}
		if task.needs_signer() {
			let Some(public_key) = self.runtime.get_task_submitter(task_id).await? else {
				tracing::debug!(
					target: TW_LOG,
					task_id,
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

	async fn execute(
		self,
		block_number: BlockNumber,
		network_id: NetworkId,
		gateway: Address,
		shard_id: ShardId,
		task_id: TaskId,
		task: Task,
	) -> Result<()> {
		let result = match task {
			Task::ReadGatewayEvents { blocks } => {
				let events =
					self.connector.read_events(gateway, blocks).await.context("read_events")?;
				tracing::info!(target: TW_LOG, task_id, "read {} events", events.len(),);
				let payload = time_primitives::encode_gmp_events(task_id, &events);
				let signature = self.tss_sign(block_number, shard_id, task_id, payload).await?;
				Some(TaskResult::ReadGatewayEvents {
					events: GmpEvents(BoundedVec::truncate_from(events)),
					signature,
				})
			},
			Task::SubmitGatewayMessage { batch_id } => {
				let msg =
					self.runtime.get_batch_message(batch_id).await?.context("invalid task")?;
				// TODO change to batch system when gaetway supports it
				let mut err_vec: ErrorMsg = ErrorMsg(BoundedVec::default());
				let msg_ops = msg.ops.clone();
				for op in msg_ops.iter() {
					// let payload = GmpParams::new(network_id, gateway).hash(&msg.encode(batch_id));
					//TODO gateway doesnt support shard management throw chronicle
					let GatewayOp::SendMessage(inner_msg) = op else {
						continue;
					};
					let payload = inner_msg.message_id();
					let signature =
						self.tss_sign(block_number, shard_id, task_id, payload.to_vec()).await?;
					let signer = self
						.runtime
						.get_shard_commitment(shard_id)
						.await?
						.context("invalid shard")?
						.0[0];
					// TODO remove when gateway support batch
					let modified_gateway_msg =
						GatewayMessage::new(vec![GatewayOp::SendMessage(inner_msg.clone())]);
					if let Err(e) = self
						.connector
						.submit_commands(gateway, batch_id, modified_gateway_msg, signer, signature)
						.await
					{
						if let Err(_) = err_vec.0.try_append(&mut e.encode()) {
							tracing::error!("Task error too long to append: {}/{}", task_id, e);
						}
					}
				}
				if err_vec.0.is_empty() {
					None
				} else {
					Some(TaskResult::SubmitGatewayMessage { error: err_vec })
				}
			},
		};
		if let Some(result) = result {
			tracing::debug!(task_id = task_id, shard_id = shard_id, "submitting task result",);
			if let Err(e) = self.runtime.submit_task_result(task_id, result).await {
				tracing::error!(
					target: TW_LOG,
					task_id = task_id,
					shard_id = shard_id,
					"Error submitting task result {:?}",
					e
				);
			}
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
		target_block_height: u64,
	) -> Result<(Vec<TaskId>, Vec<TaskId>, u64)> {
		let span = span!(
			target: TW_LOG,
			Level::DEBUG,
			"process_tasks",
			block = block_hash.to_string(),
			block_number,
		);
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
			if !self.params.is_executable(task_id, &task, target_block_height).await? {
				continue;
			}

			tracing::info!(
				task_id,
				%task,
				target_block_height,
				"Starting task"
			);
			let exec = self.params.clone();
			let handle = tokio::task::spawn(async move {
				match exec.execute(block_number, network, gateway, shard_id, task_id, task).await {
					Ok(()) => {
						tracing::info!(task_id, target_block_height, "Task completed");
					},
					Err(error) => {
						*total_failed.lock().await += 1;
						tracing::error!(task_id, target_block_height, ?error, "Task failed");
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
						target: TW_LOG,
						parent: &span,
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
