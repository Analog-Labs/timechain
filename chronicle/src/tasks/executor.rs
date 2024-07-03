use crate::TW_LOG;
use crate::{metrics::TaskPhaseCounter, tasks::TaskSpawner};
use alloy_primitives::{Address, B256};
use alloy_sol_types::{SolCall, SolEvent, SolValue};
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{
	query::GetLogs, AtBlock, CallContract, CallResult, FilterBlockOption, TransactionReceipt,
};
use std::{collections::BTreeMap, pin::Pin, sync::Arc};
use time_primitives::IGateway;
use time_primitives::{
	BlockHash, BlockNumber, Function, GmpMessage, GmpParams, Message, NetworkId, Runtime, ShardId,
	TaskExecution, TaskPhase, TssId,
};
use tokio::task::JoinHandle;

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<S, T> {
	pub wallet: Arc<Wallet>,
	pub substrate: S,
	pub task_spawner: T,
	pub network: NetworkId,
}

pub struct TaskExecutor<S, T> {
	wallet: Arc<Wallet>,
	substrate: S,
	task_spawner: T,
	network: NetworkId,
	running_tasks: BTreeMap<TaskExecution, JoinHandle<()>>,
	task_counter_metric: TaskPhaseCounter,
}

impl<S: Clone, T: Clone> Clone for TaskExecutor<S, T> {
	fn clone(&self) -> Self {
		Self {
			wallet: Arc::clone(&self.wallet),
			substrate: self.substrate.clone(),
			task_spawner: self.task_spawner.clone(),
			network: self.network,
			running_tasks: Default::default(),
			task_counter_metric: self.task_counter_metric.clone(),
		}
	}
}

///
/// implementation for TaskExecutor trait which helps in preprocessing of task
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
	) -> Result<(Vec<TssId>, Vec<TssId>)> {
		tracing::span!(target: TW_LOG, tracing::Level::INFO, "process_tasks", shard_id, block_number, target_block_height);
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
			wallet,
			substrate,
			task_spawner,
			network,
		} = params;
		Self {
			wallet,
			substrate,
			task_spawner,
			network,
			running_tasks: Default::default(),
			task_counter_metric: TaskPhaseCounter::new(),
		}
	}

	///
	/// preprocesses the task before sending it for execution in task_spawner.rs
	pub async fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<(Vec<TssId>, Vec<TssId>)> {
		// get task metadata from runtime
		let mut start_sessions = vec![];
		let tasks = self.substrate.get_shard_tasks(block_hash, shard_id).await?;
		tracing::debug!("debug_latency Current Tasks Under processing: {:?}", tasks);
		for executable_task in tasks.iter().clone() {
			tracing::debug!("debug_latency:{} in execution", executable_task);
			let task_id = executable_task.task_id;
			if self.running_tasks.contains_key(executable_task) {
				continue;
			}
			// gets task details
			let task_descr = self.substrate.get_task(block_hash, task_id).await?.unwrap();
			let target_block_number = task_descr.start;
			if target_block_height < target_block_number {
				tracing::debug!(
					"Task {} is scheduled for future {:?}/{:?}",
					task_id,
					target_block_height,
					target_block_number
				);
				continue;
			}

			let function = task_descr.function;

			// To avoid cloning the whole function later on for metric update
			let function_metric_clone = function.clone();
			let phase: TaskPhase = executable_task.phase;
			tracing::info!(
				task_id = executable_task.task_id,
				task_phase = ?executable_task.phase,
				%function,
				target_block_height,
				target_block_number,
				"Starting task"
			);
			let task = match phase {
				TaskPhase::Sign => {
					let Some(gmp_params) = self.gmp_params(shard_id, block_hash).await? else {
						tracing::warn!(
							"gmp not configured for {shard_id:?}, skipping task {task_id}"
						);
						continue;
					};
					let payload = match function {
						Function::RegisterShard { shard_id } => {
							let tss_public_key = self
								.substrate
								.get_shard_commitment(block_hash, shard_id)
								.await?
								.unwrap()[0];
							Message::update_keys([], [tss_public_key]).to_eip712_bytes(&gmp_params)
						},
						Function::UnregisterShard { shard_id } => {
							let tss_public_key = self
								.substrate
								.get_shard_commitment(block_hash, shard_id)
								.await?
								.unwrap()[0];
							Message::update_keys([tss_public_key], []).to_eip712_bytes(&gmp_params)
						},
						Function::SendMessage { msg } => {
							let msg = Message::gmp(msg);
							if let Message::Gmp(gmp) = &msg {
								match self.check_gmp_status(&gmp_params, gmp).await {
									Ok(None) => {},
									Ok(Some((receipt, gmp_executed))) => {
										// TODO: Skip the signing phase and go directly to the write phase.
										//
										// For now we will continue with the signing phase, but probably the tx will revert
										let gmp_id = gmp.gmp_id(gmp_params.gateway_contract);
										let tx_hash = receipt.transaction_hash;
										tracing::warn!("GMP message already executed while the task is in the signing phase, gmp: {gmp_id:?}, tx_hash: {tx_hash:?}, executor: {:?}", gmp_executed.status, tx_hash = tx_hash, gmp_id = gmp_id);
									},
									Err(e) => {
										// GMP status check failed
										tracing::warn!("{e:?}");
									},
								}
							}
							msg.to_eip712_bytes(&gmp_params)
						},
						_ => anyhow::bail!("invalid task"),
					};
					self.task_spawner.execute_sign(shard_id, task_id, payload.into(), block_number)
				},
				TaskPhase::Write => {
					let Some(public_key) = self.substrate.get_task_signer(task_id).await? else {
						tracing::warn!("no signer set for write phase");
						continue;
					};
					if &public_key != self.substrate.public_key() {
						continue;
					}
					let gmp_params = self.gmp_params(shard_id, block_hash).await?;
					if gmp_params.is_none() && function.initial_phase() == TaskPhase::Sign {
						tracing::warn!(
							"gmp not configured for {shard_id:?}, skipping task {task_id}"
						);
						continue;
					}

					tracing::info!("Running write phase {}", task_id);

					let function = match function {
						Function::RegisterShard { shard_id } => {
							if let Some(gmp_params) = gmp_params {
								let tss_public_key = self
									.substrate
									.get_shard_commitment(block_hash, shard_id)
									.await?
									.unwrap()[0];
								let Some(tss_signature) =
									self.substrate.get_task_signature(task_id).await?
								else {
									anyhow::bail!("tss signature not found for task {task_id}");
								};
								Message::update_keys([], [tss_public_key])
									.into_evm_call(&gmp_params, tss_signature)
							} else {
								// not gonna hit here since we already continue on is_gmp check
								anyhow::bail!(
									"gmp not configured for {shard_id:?}, skipping task {task_id}"
								)
							}
						},
						Function::UnregisterShard { shard_id } => {
							if let Some(gmp_params) = gmp_params {
								let tss_public_key = self
									.substrate
									.get_shard_commitment(block_hash, shard_id)
									.await?
									.unwrap()[0];
								let Some(tss_signature) =
									self.substrate.get_task_signature(task_id).await?
								else {
									anyhow::bail!("tss signature not found for task {task_id}");
								};
								Message::update_keys([tss_public_key], [])
									.into_evm_call(&gmp_params, tss_signature)
							} else {
								// not gonna hit here since we already continue on is_gmp check
								anyhow::bail!(
									"gmp not configured for {shard_id:?}, skipping task {task_id}"
								)
							}
						},
						Function::SendMessage { msg } => {
							if let Some(gmp_params) = gmp_params {
								let Some(tss_signature) =
									self.substrate.get_task_signature(task_id).await?
								else {
									anyhow::bail!("tss signature not found for task {task_id}");
								};
								Message::gmp(msg).into_evm_call(&gmp_params, tss_signature)
							} else {
								// not gonna hit here since we already continue on is_gmp check
								anyhow::bail!(
									"gmp not configured for {shard_id:?}, skipping task {task_id}"
								)
							}
						},
						_ => function,
					};
					self.task_spawner.execute_write(task_id, function)
				},
				TaskPhase::Read => {
					tracing::debug!("debug_latency:{} getting task_hash", task_id);
					let function = if let Some(tx) = self.substrate.get_task_hash(task_id).await? {
						Function::EvmTxReceipt { tx }
					} else {
						function
					};
					self.task_spawner.execute_read(
						target_block_number,
						shard_id,
						task_id,
						function,
						block_number,
					)
				},
			};

			// Metrics: Increase number of running tasks
			self.task_counter_metric.inc(&phase, &function_metric_clone);
			let counter = self.task_counter_metric.clone();
			start_sessions.push(TssId::new(task_id, phase));

			let handle = tokio::task::spawn(async move {
				match task.await {
					Ok(()) => {
						tracing::info!(
							task_id,
							task_phase = ?phase,
							%function_metric_clone,
							target_block_height,
							target_block_number,
							"Task completed"
						);
					},
					Err(error) => {
						tracing::error!(
							task_id,
							task_phase = ?phase,
							%function_metric_clone,
							target_block_height,
							target_block_number,
							?error,
							"Task failed"
						);
					},
				};

				// Metrics: Decrease number of running tasks
				counter.dec(&phase, &function_metric_clone);
			});
			self.running_tasks.insert(executable_task.clone(), handle);
		}
		let mut completed_sessions = Vec::with_capacity(self.running_tasks.len());
		// remove from running task if task is completed or we dont receive anymore from pallet
		self.running_tasks.retain(|x, handle| {
			if tasks.contains(x) {
				true
			} else {
				if !handle.is_finished() {
					tracing::debug!("Task {} aborted", x.task_id);
					handle.abort();
				}
				completed_sessions.push(TssId::new(x.task_id, x.phase));
				false
			}
		});
		Ok((start_sessions, completed_sessions))
	}

	///
	/// build gmp params that are needed to encode the call into evm standard
	pub async fn gmp_params(
		&self,
		shard_id: ShardId,
		block_hash: BlockHash,
	) -> Result<Option<GmpParams>> {
		let Some(gateway_contract) = self.substrate.get_gateway(self.network).await? else {
			return Ok(None);
		};
		let Some(commitment) = self.substrate.get_shard_commitment(block_hash, shard_id).await?
		else {
			tracing::error!(target: "chronicle", "shard commitment is empty for shard: {shard_id}");
			return Ok(None);
		};
		Ok(Some(GmpParams {
			network_id: self.network,
			tss_public_key: commitment[0],
			gateway_contract: gateway_contract.into(),
		}))
	}

	///
	/// Retrieve gmp executed event from gateway contract
	pub async fn check_gmp_status(
		&self,
		params: &GmpParams,
		message: &GmpMessage,
	) -> Result<Option<(TransactionReceipt, IGateway::GmpExecuted)>> {
		// Check if chronicle network and GMP destination network are the same
		if message.destNetwork != self.network {
			let dest_gateway = self.substrate.get_gateway(message.destNetwork).await.ok().flatten();
			if let Some(dest_gateway) = dest_gateway {
				let gmp_id = message.gmp_id(dest_gateway.into());
				anyhow::bail!("[report this bug] invalid GMP network, expect {} got {} for gmp_id: {gmp_id:?}", self.network, message.destNetwork);
			} else {
				anyhow::bail!(
					"[report this bug] no gateway contract registered for network {}",
					message.destNetwork
				);
			}
		}

		// Compute GMP ID
		let gmp_id = message.gmp_id(params.gateway_contract);

		// Check if the GMP message was already executed
		match self.get_gmp_executed_tx_receipt(params.gateway_contract, gmp_id).await {
			Ok(None) => Ok(None),
			Ok(Some((receipt, other))) => Ok(Some((receipt, other))),
			Err(e) => {
				tracing::warn!("Failed to check status of the gmp status for '{gmp_id:?}': {e:?}");
				Ok(None)
			},
		}
	}

	///
	/// Retrieve gmp executed event from gateway contract
	pub async fn get_gmp_executed_tx_receipt(
		&self,
		gateway_contract: Address,
		gmp_id: B256,
	) -> Result<Option<(TransactionReceipt, IGateway::GmpExecuted)>> {
		// Query the GmpInfo from the gateway contract
		let gmp_info = self
			.wallet
			.query(CallContract {
				from: None,
				to: gateway_contract.into_array().into(),
				value: 0u64.into(),
				data: IGateway::gmpInfoCall { id: gmp_id }.abi_encode(),
				block: AtBlock::Latest,
			})
			.await?;
		if let Some(revert_message) = gmp_info.revert_msg() {
			anyhow::bail!("gmp info call reverted: {revert_message:?}");
		}
		let gmp_info = match gmp_info {
			CallResult::Success(gmp_info) => IGateway::GmpInfo::abi_decode(&gmp_info, true)?,
			CallResult::Revert(bytes) => {
				anyhow::bail!("gmp info call failed with error: {:?}", hex::encode(bytes));
			},
			CallResult::Error => {
				anyhow::bail!("gmp info call failed with error");
			},
		};
		if gmp_info.status == IGateway::GmpStatus::NOT_FOUND {
			return Ok(None);
		}

		// Query the GmpExecuted event from the gateway contract
		let maybe_gmp_executed = self
			.wallet
			.query(GetLogs {
				contracts: vec![gateway_contract.into_array().into()],
				topics: vec![IGateway::GmpExecuted::SIGNATURE_HASH.0.into(), gmp_id.0.into()],
				block: FilterBlockOption::Range {
					from_block: None,
					to_block: Some(gmp_info.blockNumber.into()),
				},
			})
			.await?
			.into_iter()
			.find_map(|log| {
				// Must have a transaction hash
				let tx_hash = log.transaction_hash.map(|tx_hash| B256::from(tx_hash.0))?;

				// if any GmpCreated message is received collect them and returns
				let topics = log.topics.iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
				let log = alloy_primitives::Log::new(
					gateway_contract.into(),
					topics,
					log.data.0.to_vec().into(),
				)?;

				// Check if the log is a GmpExecuted event
				if matches!(log.topics().first(), Some(&IGateway::GmpExecuted::SIGNATURE_HASH)) {
					return None;
				}

				// Decode the GmpExecuted event
				let gmp_executed_event = match IGateway::GmpExecuted::decode_log(&log, true) {
					Ok(log) => log.data,
					Err(err) => {
						tracing::warn!(
							"Failed to decode GmpExecuted for tx {tx_hash:?}, err: {err:?}"
						);
						return None;
					},
				};

				Option::Some((tx_hash, gmp_executed_event))
			});

		// Retrieve tx receipt for the GmpExecuted event
		let Some((tx_hash, gmp_executed)) = maybe_gmp_executed else {
			return Ok(None);
		};

		let Some(tx_receipt) = self.wallet.eth_transaction_receipt(tx_hash.0).await? else {
			// This should not happen, but if it does, log a warning and return None
			tracing::warn!("Transaction receipt for GmpExecuted not found, gmp_id: '{gmp_id:?}', tx_hash: '{tx_hash:?}'");
			return Ok(None);
		};

		Result::Ok(Some((tx_receipt, gmp_executed)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::Mock;
	use futures::StreamExt;
	use rosetta_client::Blockchain;
	use time_primitives::{Msg, TaskDescriptor};

	#[tokio::test]
	async fn task_executor_smoke() -> Result<()> {
		env_logger::try_init().ok();

		let mock = Mock::default().instance(42);
		let wallet = Arc::new(
			Wallet::new(
				Blockchain::Ethereum,
				"dev".into(),
				"http://localhost:8545".into(),
				None,
				None,
			)
			.await
			.unwrap(),
		);

		let network = mock.create_network("ethereum".into(), "dev".into());
		let shard = mock.create_online_shard(vec![mock.account_id().clone()], 1);
		let task = mock.create_task(TaskDescriptor {
			owner: Some(mock.account_id().clone()),
			network,
			function: Function::SendMessage { msg: Msg::default() },
			start: 0,
			shard_size: 1,
		});
		mock.assign_task(task, shard);
		let target_block_height = 0;

		let params = TaskExecutorParams {
			wallet,
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
			if task.result.is_none() {
				tracing::info!("task phase {:?}", task.phase);
				continue;
			}
			break;
		}
		Ok(())
	}
}
