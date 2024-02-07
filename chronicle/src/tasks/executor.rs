use crate::gmp::MessageBuilder;
use crate::tasks::TaskSpawner;
use crate::TW_LOG;
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::{collections::BTreeMap, pin::Pin};
use time_primitives::{
	BlockHash, BlockNumber, Function, NetworkId, Runtime, ShardId, TaskExecution, TaskPhase, TssId,
};
use tokio::task::JoinHandle;

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<S, T> {
	pub substrate: S,
	pub task_spawner: T,
	pub network: NetworkId,
}

pub struct TaskExecutor<S, T> {
	substrate: S,
	task_spawner: T,
	network: NetworkId,
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

#[async_trait]
impl<S, T> super::TaskExecutor for TaskExecutor<S, T>
where
	S: Runtime,
	T: TaskSpawner + Send + Sync,
{
	fn network(&self) -> NetworkId {
		self.network
	}

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		self.task_spawner.block_stream()
	}

	async fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<Vec<TssId>> {
		TaskExecutor::process_tasks(self, block_hash, block_number, shard_id, target_block_height)
			.await
	}
}

impl<S, T> TaskExecutor<S, T>
where
	S: Runtime,
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

	pub async fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<Vec<TssId>> {
		let tasks = self.substrate.get_shard_tasks(block_hash, shard_id).await?;
		tracing::info!(target: TW_LOG, "got task ====== {:?}", tasks);
		for executable_task in tasks.iter().clone() {
			let task_id = executable_task.task_id;
			let cycle = executable_task.cycle;
			let retry_count = executable_task.retry_count;
			if self.running_tasks.contains_key(executable_task) {
				tracing::info!(target: TW_LOG, "skipping task {:?}", task_id);
				continue;
			}
			let task_descr = self.substrate.get_task(block_hash, task_id).await?.unwrap();
			let target_block_number = task_descr.trigger(cycle).ok_or(anyhow::anyhow!(
				"Overflow while calculating target block: {}/{}",
				task_id,
				cycle
			))?;
			let function = task_descr.function;
			let hash = task_descr.timegraph;
			if target_block_height >= target_block_number {
				tracing::info!(target: TW_LOG, "Running Task {}, {:?}", executable_task, executable_task.phase);
				let task = if matches!(executable_task.phase, TaskPhase::Sign) {
					let Some(msg_builder) = self.gmp_builder_for(shard_id, block_hash).await?
					else {
						tracing::warn!(
							"gmp not configured for {shard_id:?}, skipping task {task_id}"
						);
						continue;
					};
					let payload = match function {
						Function::RegisterShard { shard_id } => {
							let tss_public_key =
								self.substrate.get_shard_commitment(block_hash, shard_id).await?[0];
							msg_builder.build_update_keys_message([], [tss_public_key]).hash()
						},
						Function::UnregisterShard { shard_id } => {
							let tss_public_key =
								self.substrate.get_shard_commitment(block_hash, shard_id).await?[0];
							msg_builder.build_update_keys_message([tss_public_key], []).hash()
						},
						Function::SendMessage {
							address,
							payload,
							salt,
							gas_limit,
						} => msg_builder.build_gmp_message(address, payload, salt, gas_limit).hash(),
						_ => anyhow::bail!("invalid task"),
					};
					self.task_spawner.execute_sign(shard_id, task_id, cycle, payload, block_number)
				} else if let Some(public_key) = executable_task.phase.public_key() {
					if public_key != self.substrate.public_key() {
						tracing::info!(target: TW_LOG, "Skipping task {} due to public_key mismatch", task_id);
						continue;
					}
					let msg_builder = self.gmp_builder_for(shard_id, block_hash).await?;

					if msg_builder.is_none() && function.is_gmp() {
						tracing::warn!(
							"gmp not configured for {shard_id:?}, skipping task {task_id}"
						);
						continue;
					}

					let function = match function {
						Function::RegisterShard { shard_id } => {
							if let Some(msg_builder) = msg_builder {
								let tss_public_key = self
									.substrate
									.get_shard_commitment(block_hash, shard_id)
									.await?[0];
								let Some(tss_signature) =
									self.substrate.get_task_signature(task_id).await?
								else {
									anyhow::bail!("tss signature not found for task {task_id}");
								};
								msg_builder
									.build_update_keys_message([], [tss_public_key])
									.into_evm_call(tss_signature)
							} else {
								// not gonna hit here since we already continue on is_gmp check
								anyhow::bail!(
									"gmp not configured for {shard_id:?}, skipping task {task_id}"
								)
							}
						},
						Function::UnregisterShard { shard_id } => {
							if let Some(msg_builder) = msg_builder {
								let tss_public_key = self
									.substrate
									.get_shard_commitment(block_hash, shard_id)
									.await?[0];
								let Some(tss_signature) =
									self.substrate.get_task_signature(task_id).await?
								else {
									anyhow::bail!("tss signature not found for task {task_id}");
								};
								msg_builder
									.build_update_keys_message([tss_public_key], [])
									.into_evm_call(tss_signature)
							} else {
								// not gonna hit here since we already continue on is_gmp check
								anyhow::bail!(
									"gmp not configured for {shard_id:?}, skipping task {task_id}"
								)
							}
						},
						Function::SendMessage {
							address,
							payload,
							salt,
							gas_limit,
						} => {
							if let Some(msg_builder) = msg_builder {
								let Some(tss_signature) =
									self.substrate.get_task_signature(task_id).await?
								else {
									anyhow::bail!("tss signature not found for task {task_id}");
								};
								msg_builder
									.build_gmp_message(address, payload, salt, gas_limit)
									.into_evm_call(tss_signature)
							} else {
								// not gonna hit here since we already continue on is_gmp check
								anyhow::bail!(
									"gmp not configured for {shard_id:?}, skipping task {task_id}"
								)
							}
						},
						_ => function,
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

	pub async fn gmp_builder_for(
		&self,
		shard_id: ShardId,
		block_hash: BlockHash,
	) -> Result<Option<MessageBuilder>> {
		let gateway_contract = {
			let Some(gateway_contract) = self.substrate.get_gateway(self.network).await? else {
				return Ok(None);
			};
			if gateway_contract.len() != 20 {
				tracing::error!(target: "chronicle", "invalid gateway contract address for network {:?}, expect 20 bytes got {}", self.network, gateway_contract.len());
				return Ok(None);
			}
			let mut output = [0u8; 20];
			output.copy_from_slice(&gateway_contract);
			output
		};
		let Some(tss_public_key) = self
			.substrate
			.get_shard_commitment(block_hash, shard_id)
			.await?
			.first()
			.copied()
		else {
			tracing::error!(target: "chronicle", "shard commitment is empty for shard: {shard_id}");
			return Ok(None);
		};
		Ok(Some(MessageBuilder::new(
			shard_id,
			self.task_spawner.chain_id(),
			tss_public_key,
			gateway_contract,
		)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Mock;
	use futures::StreamExt;
	use time_primitives::{TaskDescriptor, TaskStatus};

	#[tokio::test]
	async fn task_executor_smoke() -> Result<()> {
		env_logger::try_init().ok();

		let mock = Mock::default().instance(42);

		let network = mock.create_network("ethereum".into(), "dev".into());
		let shard = mock.create_online_shard(vec![mock.account_id().clone()], 1);
		let task = mock.create_task(TaskDescriptor {
			owner: Some(mock.account_id().clone()),
			network,
			cycle: 1,
			function: Function::SendMessage {
				address: Default::default(),
				gas_limit: Default::default(),
				salt: Default::default(),
				payload: Default::default(),
			},
			period: 0,
			start: 0,
			timegraph: None,
			shard_size: 2,
		});
		mock.assign_task(task, shard);
		let target_block_height = 0;

		let params = TaskExecutorParams {
			task_spawner: mock.clone(),
			network,
			substrate: mock.clone(),
		};
		let mut task_executor = TaskExecutor::new(params);
		while let Some((block_hash, block_number)) =
			mock.finality_notification_stream().next().await
		{
			task_executor
				.process_tasks(block_hash, block_number, shard, target_block_height)
				.await
				.unwrap();
			tracing::info!("Watching for result");
			let task = mock.task(task).unwrap();
			if task.status != TaskStatus::Completed {
				tracing::info!("task phase {:?}", task.phase);
				continue;
			}
			break;
		}
		Ok(())
	}
}
