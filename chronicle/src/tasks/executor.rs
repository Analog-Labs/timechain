use crate::tasks::TaskSpawner;
use crate::TW_LOG;
use anyhow::Result;
use futures::Stream;
use std::{collections::BTreeMap, pin::Pin};
use time_primitives::{
	BlockHash, BlockNumber, Function, Network, PublicKey, ShardId, TaskExecution, Tasks, TssId,
};
use tokio::task::JoinHandle;

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<S, T>
where
	S: Tasks + Clone + Send + Sync + 'static,
	T: TaskSpawner + Send + Sync + 'static,
{
	pub substrate: S,
	pub task_spawner: T,
	pub network: Network,
	pub public_key: PublicKey,
}

pub struct TaskExecutor<S, T> {
	substrate: S,
	task_spawner: T,
	network: Network,
	public_key: PublicKey,
	running_tasks: BTreeMap<TaskExecution, JoinHandle<()>>,
}

impl<S, T> Clone for TaskExecutor<S, T>
where
	S: Tasks + Clone + Send + Sync + 'static,
	T: TaskSpawner + Send + Sync + Clone + 'static,
{
	fn clone(&self) -> Self {
		Self {
			substrate: self.substrate.clone(),
			task_spawner: self.task_spawner.clone(),
			network: self.network,
			public_key: self.public_key.clone(),
			running_tasks: Default::default(),
		}
	}
}

impl<S, T> super::TaskExecutor for TaskExecutor<S, T>
where
	S: Tasks + Clone + Send + Sync + 'static,
	T: TaskSpawner + Send + Sync + 'static,
{
	fn network(&self) -> Network {
		self.network
	}

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		self.task_spawner.block_stream()
	}

	fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<Vec<TssId>> {
		self.process_tasks(block_hash, block_number, shard_id, target_block_height)
	}
}

impl<S, T> TaskExecutor<S, T>
where
	S: Tasks + Clone + Send + Sync + 'static,
	T: TaskSpawner + Send + Sync + 'static,
{
	pub fn new(params: TaskExecutorParams<S, T>) -> Self {
		let TaskExecutorParams {
			substrate,
			task_spawner,
			network,
			public_key,
		} = params;
		Self {
			substrate,
			task_spawner,
			network,
			public_key,
			running_tasks: Default::default(),
		}
	}

	pub fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<Vec<TssId>> {
		let tasks = self.substrate.get_shard_tasks(block_hash, shard_id)?;
		tracing::info!(target: TW_LOG, "got task ====== {:?}", tasks);
		for executable_task in tasks.iter().clone() {
			let task_id = executable_task.task_id;
			let cycle = executable_task.cycle;
			let retry_count = executable_task.retry_count;
			if self.running_tasks.contains_key(executable_task) {
				tracing::info!(target: TW_LOG, "skipping task {:?}", task_id);
				continue;
			}
			let task_descr = self.substrate.get_task(block_hash, task_id)?.unwrap();
			let target_block_number = task_descr.trigger(cycle);
			let function = task_descr.function;
			let hash = task_descr.hash;
			if target_block_height >= target_block_number {
				tracing::info!(target: TW_LOG, "Running Task {}, {:?}", executable_task, executable_task.phase);
				let task = if let Some(public_key) = executable_task.phase.public_key() {
					if *public_key != self.public_key {
						tracing::info!(target: TW_LOG, "Skipping task {} due to public_key mismatch", task_id);
						continue;
					}
					self.task_spawner.execute_write(task_id, cycle, function)
				} else {
					let function = if let Some(tx) = executable_task.phase.tx_hash() {
						Function::EvmTxReceipt { tx: tx.to_vec() }
					} else {
						function
					};
					self.task_spawner.execute_read(
						target_block_number,
						shard_id,
						task_id,
						cycle,
						function,
						hash,
						block_number,
					)
				};
				let handle = tokio::task::spawn(async move {
					match task.await {
						Ok(()) => {
							tracing::info!(
								target: TW_LOG,
								"Task {}/{}/{} completed",
								task_id,
								cycle,
								retry_count,
							);
						},
						Err(error) => {
							tracing::error!(
								target: TW_LOG,
								"Task {}/{}/{} failed {:?}",
								task_id,
								cycle,
								retry_count,
								error,
							);
						},
					}
				});
				self.running_tasks.insert(executable_task.clone(), handle);
			} else {
				tracing::info!(
					"Task is scheduled for future {:?}/{:?}/{:?}",
					task_id,
					target_block_height,
					target_block_number
				);
			}
		}
		let mut completed_sessions = Vec::with_capacity(self.running_tasks.len());
		self.running_tasks.retain(|x, handle| {
			if tasks.contains(x) {
				true
			} else {
				if !handle.is_finished() {
					tracing::info!(target: TW_LOG, "Task {}/{}/{} aborted", x.task_id, x.cycle, x.retry_count);
					handle.abort();
				}
				completed_sessions.push(TssId(x.task_id, x.cycle));
				false
			}
		});
		Ok(completed_sessions)
	}
}
