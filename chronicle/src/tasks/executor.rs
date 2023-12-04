use crate::substrate::SubstrateClient;
use crate::tasks::TaskSpawner;
use crate::TW_LOG;
use anyhow::Result;
use futures::Stream;
use std::{collections::BTreeMap, pin::Pin};
use time_primitives::{
	BlockHash, BlockNumber, Function, Network, ShardId, TaskExecution, TaskPhase, Tasks, TssId,
	WrappedGmpMessage,
};
use tokio::task::JoinHandle;

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<S, T> {
	pub substrate: S,
	pub task_spawner: T,
	pub network: Network,
}

pub struct TaskExecutor<S, T> {
	substrate: S,
	task_spawner: T,
	network: Network,
	running_tasks: BTreeMap<TaskExecution, JoinHandle<()>>,
}

impl<S: Clone, T: Clone> Clone for TaskExecutor<S, T> {
	fn clone(&self) -> Self {
		Self {
			substrate: self.substrate.clone(),
			task_spawner: self.task_spawner.clone(),
			network: self.network,
			running_tasks: Default::default(),
		}
	}
}

impl<S, T> super::TaskExecutor for TaskExecutor<S, T>
where
	S: Tasks + SubstrateClient,
	T: TaskSpawner,
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
	S: Tasks + SubstrateClient,
	T: TaskSpawner,
{
	pub fn new(params: TaskExecutorParams<S, T>) -> Self {
		let TaskExecutorParams {
			substrate,
			task_spawner,
			network,
		} = params;
		Self {
			substrate,
			task_spawner,
			network,
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
				let task = if matches!(executable_task.phase, TaskPhase::Sign) {
					let Function::SendMessage { payload, .. } = function else {
						continue; // create_task ensures never hits this branch
						 // by only setting TaskPhase::Sign iff function == Function::SendMessage
					};
					self.task_spawner.execute_sign(shard_id, task_id, cycle, payload, block_number)
				} else if let Some(public_key) = executable_task.phase.public_key() {
					if *public_key != self.substrate.public_key() {
						tracing::info!(target: TW_LOG, "Skipping task {} due to public_key mismatch", task_id);
						continue;
					}
					let function = if let Function::SendMessage { contract_address, payload } =
						function
					{
						let _signature = self.substrate.get_task_signature(task_id)?.unwrap();
						let gmp_message: WrappedGmpMessage =
							bincode::deserialize(&payload).unwrap();
						Function::EvmCall {
								address: String::from_utf8(contract_address.clone()).unwrap(),
								//TODO right now it doesnt work, because connector doesnt support custom structs
								function_signature: String::from("rawSudoExecute(bytes32,uint128,address,uint128,uint256,uint256,bytes)"),
								input: vec![
									hex::encode(gmp_message.source),
									format!("{:?}", gmp_message.src_network),
									hex::encode(gmp_message.dest),
									format!("{:?}", gmp_message.dest_network),
									gmp_message.gas_limit.to_string(),
									gmp_message.salt.to_string(),
									hex::encode(gmp_message.data)
								],
								// TODO estimate gas required for gateway
								amount: 0, // >0 so failed execution is not due to lack of gas
							}
					} else if let Function::EvmCall {
						address,
						function_signature,
						input,
						amount,
					} = function
					{
						if let Ok(Some(signature)) = self.substrate.get_task_signature(task_id) {
							Function::EvmCall {
								address,
								function_signature,
								input: time_primitives::insert_sig_iff_register_shard(
									input, signature,
								),
								amount,
							}
						} else {
							function
						}
					} else {
						function
					};
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
