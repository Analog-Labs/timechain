use crate::{TaskExecutorParams, TW_LOG};
use anyhow::{anyhow, Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, StreamExt};
use rosetta_client::{create_wallet, types::PartialBlockIdentifier, EthereumExt, Wallet};
use sc_client_api::{Backend, BlockchainEvents, HeaderBackend};
use serde_json::Value;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_runtime::traits::Block;
use std::{
	collections::{HashMap, HashSet},
	future::Future,
	marker::PhantomData,
	pin::Pin,
	sync::Arc,
};
use time_primitives::{
	CycleStatus, Function, OcwPayload, PeerId, ShardId, TaskCycle, TaskDescriptor, TaskError,
	TaskExecution, TaskId, TaskSpawner, TimeApi, TssRequest, TssSignature,
};
use timegraph_client::{Timegraph, TimegraphData};

pub struct TaskSpawnerParams {
	pub tss: mpsc::Sender<TssRequest>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
}

#[derive(Clone)]
pub struct Task {
	tss: mpsc::Sender<TssRequest>,
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
		Ok(Self {
			tss: params.tss,
			wallet,
			timegraph,
		})
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

	async fn tss_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		result: &[String],
	) -> Result<TssSignature> {
		let data = bincode::serialize(&result).context("Failed to serialize task")?;
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssRequest {
				request_id: (task_id, cycle),
				shard_id,
				data,
				tx,
			})
			.await?;
		Ok(rx.await?)
	}

	async fn execute(
		self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		task_cycle: TaskCycle,
		task: TaskDescriptor,
		block_num: u64,
	) -> Result<TssSignature> {
		let result = self
			.execute_function(&task.function, target_block)
			.await
			.with_context(|| format!("Failed to execute {:?}", task.function))?;
		let signature = self
			.tss_sign(shard_id, task_id, task_cycle, &result)
			.await
			.with_context(|| format!("Failed to tss sign on shard {}", shard_id))?;
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
		task: TaskDescriptor,
		block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>> {
		self.clone()
			.execute(target_block, shard_id, task_id, cycle, task, block_num)
			.map(move |res| res.with_context(|| format!("Task {}/{} failed", task_id, cycle)))
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
	execution_block_map: HashMap<TaskExecution, u64>,
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
			execution_block_map: Default::default(),
			task_spawner,
		}
	}

	pub async fn start_tasks(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let block_height =
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
				if self.running_tasks.contains(&executable_task) {
					if let Some(executed_block_num) = self.execution_block_map.get(&executable_task)
					{
						if executed_block_num + 30 < block_num {
							log::warn!("clearing execution for task {:?}", executable_task.task_id);
							self.running_tasks.remove(&executable_task);
						}
					}
					continue;
				}
				let task_descr = self.runtime.runtime_api().get_task(block_id, task_id)?.unwrap();
				let target_block_number = task_descr.trigger(cycle);
				if block_height >= target_block_number {
					log::info!(target: TW_LOG, "Running Task {} on shard {}", executable_task, shard_id);
					self.running_tasks.insert(executable_task);
					self.execution_block_map.insert(executable_task, block_num);
					let storage = self.backend.offchain_storage().unwrap();
					let task = self.task_spawner.execute(
						target_block_number,
						shard_id,
						task_id,
						cycle,
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
									&OcwPayload::SubmitTaskResult { task_id, cycle, status },
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
