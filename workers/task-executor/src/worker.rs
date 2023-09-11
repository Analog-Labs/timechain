use crate::{TaskExecutorParams, TW_LOG};
use anyhow::{anyhow, Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, StreamExt};
use rosetta_client::{create_wallet, types::PartialBlockIdentifier, EthereumExt, Wallet};
use sc_client_api::{Backend, BlockchainEvents, HeaderBackend};
use serde_json::Value;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_runtime::traits::Block;
use std::{collections::HashSet, future::Future, marker::PhantomData, pin::Pin, sync::Arc};
use time_primitives::{
	CycleStatus, Function, OcwPayload, PeerId, ShardId, TaskCycle, TaskDescriptor, TaskError,
	TaskExecution, TaskId, TaskRetryCount, TaskSpawner, TimeApi, TssSignature,
};
use timegraph_client::{Timegraph, TimegraphData};

pub struct TaskSpawnerParams {
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
}

#[derive(Clone)]
pub struct Task {
	wallet: Arc<Wallet>,
	timegraph: Option<Arc<Timegraph>>,
}

impl Task {
	pub async fn new(params: TaskSpawnerParams) -> Result<Self> {
		let wallet = Arc::new(
			create_wallet(
				params.connector_blockchain,
				params.connector_network,
				params.connector_url,
				None,
			)
			.await?,
		);
		let timegraph = if let Some(url) = params.timegraph_url {
			Some(Arc::new(Timegraph::new(
				url,
				params
					.timegraph_ssk
					.as_deref()
					.ok_or(anyhow!("timegraph session key is not specified"))?
					.to_string(),
			)?))
		} else {
			None
		};
		Ok(Self { wallet, timegraph })
	}

	async fn execute_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<Vec<String>> {
		let block = PartialBlockIdentifier {
			index: Some(target_block_number),
			hash: None,
		};
		match function {
			Function::EVMViewWithoutAbi {
				address,
				function_signature,
				input,
			} => {
				let data = self
					.wallet
					.eth_view_call(address, function_signature, input, Some(block))
					.await?;
				let result = match data.result {
					Value::Array(val) => val
						.iter()
						.filter_map(|x| x.as_str())
						.map(|x| x.to_string())
						.collect::<Vec<String>>(),
					v => vec![v.to_string()],
				};
				Ok(result)
			},
		}
	}

	async fn execute(
		self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		task_cycle: TaskCycle,
		retry_count: TaskRetryCount,
		task: TaskDescriptor,
		block_num: u64,
	) -> Result<TssSignature> {
		let result = self
			.execute_function(&task.function, target_block)
			.await
			.with_context(|| format!("Failed to execute {:?}", task.function))?;
		let signature = [0; 64];
		if let Some(timegraph) = self.timegraph.as_ref() {
			timegraph
				.submit_data(TimegraphData {
					collection: task.hash.clone(),
					task_id,
					task_cycle,
					target_block_number: target_block,
					timechain_block_number: block_num,
					shard_id,
					signature,
					data: result,
				})
				.await
				.context("Failed to submit data to timegraph")?;
		}
		Ok(signature)
	}
}

#[async_trait::async_trait]
impl TaskSpawner for Task {
	async fn block_height(&self) -> Result<u64> {
		let status = self.wallet.status().await?;
		Ok(status.index)
	}

	fn execute(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		retry_count: TaskRetryCount,
		task: TaskDescriptor,
		block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>> {
		self.clone()
			.execute(target_block, shard_id, task_id, cycle, retry_count, task, block_num)
			.map(move |res| {
				res.with_context(|| format!("Task {}/{}/{} failed", task_id, cycle, retry_count))
			})
			.boxed()
	}
}

pub struct TaskExecutor<B: Block, BE, C, R, T> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	client: Arc<C>,
	runtime: Arc<R>,
	peer_id: PeerId,
	running_tasks: HashSet<TaskExecution>,
	task_spawner: T,
}

impl<B, BE, C, R, T> TaskExecutor<B, BE, C, R, T>
where
	B: Block,
	BE: Backend<B> + 'static,
	C: BlockchainEvents<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
	T: TaskSpawner,
{
	pub fn new(params: TaskExecutorParams<B, BE, C, R, T>) -> Self {
		let TaskExecutorParams {
			_block,
			backend,
			client,
			runtime,
			peer_id,
			task_spawner,
		} = params;
		Self {
			_block,
			backend,
			client,
			runtime,
			peer_id,
			running_tasks: Default::default(),
			task_spawner,
		}
	}

	pub async fn start_tasks(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let block_height: u64 =
			self.task_spawner.block_height().await.context("Failed to fetch block height")?;
		let shards = self.runtime.runtime_api().get_shards(block_id, self.peer_id)?;
		let block_num = self.backend.blockchain().number(block_id)?.unwrap();
		let block_num: u64 = block_num.to_string().parse()?;
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id)?;
			log::info!(target: TW_LOG, "got task ====== {:?}", tasks);
			for executable_task in tasks.iter().copied() {
				let task_id = executable_task.task_id;
				let cycle = executable_task.cycle;
				let retry_count = executable_task.retry_count;
				let last_executed_block = executable_task.last_block_num;
				if self.running_tasks.contains(&executable_task) {
					log::info!("skipping task {:?}", executable_task);
					continue;
				}
				let task_descr = self.runtime.runtime_api().get_task(block_id, task_id)?.unwrap();
				let target_block_number = if let Some(last_block) = last_executed_block {
					last_block + task_descr.period
				} else {
					task_descr.trigger(cycle)
				};
				if block_height >= target_block_number {
					log::info!(target: TW_LOG, "Running Task {} on shard {}", executable_task, shard_id);
					self.running_tasks.insert(executable_task);
					let storage = self.backend.offchain_storage().unwrap();
					let task = self.task_spawner.execute(
						target_block_number,
						shard_id,
						task_id,
						cycle,
						retry_count,
						task_descr,
						block_num,
					);
					tokio::task::spawn(async move {
						let result = task.await.map_err(|e| format!("{:?}", e));
						log::info!(
							target: TW_LOG,
							"Task {} completed on shard {} with {:?}",
							executable_task,
							shard_id,
							result
						);
						match result {
							Ok(signature) => {
								let status = CycleStatus { shard_id, signature };
								time_primitives::write_message(
									storage,
									&OcwPayload::SubmitTaskResult {
										task_id,
										cycle,
										status,
										block: target_block_number,
									},
								);
							},
							Err(error) => {
								let error = TaskError { shard_id, error };
								time_primitives::write_message(
									storage,
									&OcwPayload::SubmitTaskError { task_id, error },
								);
							},
						}
					});
				} else {
					log::info!(
						"block_height < target_block_number === {} < {}",
						block_height,
						target_block_number
					);
				}
			}
			self.running_tasks.retain(|x| tasks.contains(x));
		}
		Ok(())
	}

	pub async fn run(&mut self) {
		let mut finality_notifications = self.client.finality_notification_stream();
		while let Some(notification) = finality_notifications.next().await {
			log::debug!(target: TW_LOG, "finalized {}", notification.header.number());
			if let Err(err) = self.start_tasks(notification.header.hash()).await {
				log::error!(target: TW_LOG, "error processing tasks: {:?}", err);
			}
		}
	}
}
