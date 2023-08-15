use crate::TaskExecutorParams;
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, StreamExt};
use rosetta_client::{create_wallet, EthereumExt, Wallet};
use sc_client_api::{Backend, BlockchainEvents, HeaderBackend};
use serde_json::Value;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_runtime::traits::Block;
use std::{collections::HashSet, future::Future, marker::PhantomData, pin::Pin, sync::Arc};
use time_primitives::{
	CycleStatus, Function, FunctionResult, OcwPayload, PeerId, ShardId, TaskCycle, TaskDescriptor,
	TaskError, TaskExecution, TaskId, TimeApi, TssRequest, TssSignature,
};

pub struct TaskSpawnerParams {
	pub tss: mpsc::Sender<TssRequest>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

#[derive(Clone)]
pub struct Task {
	tss: mpsc::Sender<TssRequest>,
	wallet: Arc<Wallet>,
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
		Ok(Self { tss: params.tss, wallet })
	}

	async fn execute_function(&self, function: &Function) -> Result<FunctionResult> {
		match function {
			Function::EVMViewWithoutAbi {
				address,
				function_signature,
				input,
			} => {
				let data = self.wallet.eth_view_call(address, function_signature, input).await?;
				let result = match data.result {
					Value::Array(val) => val
						.iter()
						.filter_map(|x| x.as_str())
						.map(|x| x.to_string())
						.collect::<Vec<String>>(),
					v => vec![v.to_string()],
				};
				Ok(FunctionResult::EVMViewWithoutAbi { result })
			},
		}
	}

	async fn tss_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		result: &FunctionResult,
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
		cycle: TaskCycle,
		task: TaskDescriptor,
		block_num: i64,
	) -> Result<TssSignature> {
		let result = self.execute_function(&task.function).await?;
		let signature = self.tss_sign(shard_id, task_id, cycle, &result).await?;
		timechain_integration::submit_to_timegraph(
			task.hash.clone(),
			task_id,
			cycle,
			target_block,
			block_num,
			signature,
			result,
		)
		.await?;
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
		cycle: ScheduleCycle,
		task: TaskSchedule,
		block_num: i64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>> {
		self.clone()
			.execute(target_block, shard_id, task_id, cycle, task, block_num)
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

	async fn start_tasks(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let block_height = self.task_spawner.block_height().await?;
		let shards = self.runtime.runtime_api().get_shards(block_id, self.peer_id)?;
		let block_num = self.backend.blockchain().number(block_id)?.unwrap();
		let block_num: i64 = block_num.to_string().parse()?;
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id)?;
			log::info!("got task ====== {:?}", tasks);
			for executable_task in tasks.iter().copied() {
				let task_id = executable_task.task_id;
				let cycle = executable_task.cycle;
				if self.running_tasks.contains(&executable_task) {
					continue;
				}
				let task_descr = self.runtime.runtime_api().get_task(block_id, task_id)?.unwrap();
				if block_height >= task_descr.trigger(cycle) {
					log::info!("Running Task {:?}", task_id);
					self.running_tasks.insert(executable_task);
					let storage = self.backend.offchain_storage().unwrap();
					let task = self.task_spawner.execute(
						block_height,
						shard_id,
						task_id,
						cycle,
						task_descr,
						block_num,
					);
					tokio::task::spawn(async move {
						let result = task.await.map_err(|e| e.to_string());

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
			if let Err(err) = self.start_tasks(notification.header.hash()).await {
				log::error!("error processing tasks: {}", err);
			}
		}
	}
}
