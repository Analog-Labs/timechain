use crate::TaskExecutorParams;
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, StreamExt};
use rosetta_client::{create_wallet, EthereumExt, Wallet};
use sc_client_api::{Backend, BlockchainEvents, HeaderBackend};
use serde_json::Value;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_runtime::traits::Block;
use std::{collections::HashSet, marker::PhantomData, sync::Arc};
use time_primitives::{
	ExecutableTask, Function, FunctionResult, OcwPayload, PeerId, ScheduleCycle, ScheduleError,
	ScheduleStatus, ShardId, TaskId, TaskSchedule, TimeApi, TssRequest, TssSignature,
};

pub struct Task {
	tss: mpsc::Sender<TssRequest>,
	wallet: Arc<Wallet>,
}

impl Task {
	pub fn new(tss: mpsc::Sender<TssRequest>, wallet: Arc<Wallet>) -> Self {
		Self { tss, wallet }
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
		cycle: ScheduleCycle,
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
		cycle: ScheduleCycle,
		task: TaskSchedule,
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

pub struct TaskExecutor<B: Block, BE, R> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	peer_id: PeerId,
	sign_data_sender: mpsc::Sender<TssRequest>,
	running_tasks: HashSet<ExecutableTask>,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
}

impl<B, BE, R> TaskExecutor<B, BE, R>
where
	B: Block,
	BE: Backend<B> + 'static,
	R: BlockchainEvents<B> + ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
{
	pub fn new(params: TaskExecutorParams<B, BE, R>) -> Self {
		let TaskExecutorParams {
			_block,
			backend,
			runtime,
			peer_id,
			sign_data_sender,
			connector_url,
			connector_blockchain,
			connector_network,
		} = params;
		Self {
			_block,
			backend,
			runtime,
			peer_id,
			sign_data_sender,
			running_tasks: Default::default(),
			connector_url,
			connector_blockchain,
			connector_network,
		}
	}

	async fn start_tasks(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let wallet = Arc::new(
			create_wallet(
				self.connector_blockchain.clone(),
				self.connector_network.clone(),
				self.connector_url.clone(),
				None,
			)
			.await?,
		);
		let status = wallet.status().await?;
		let block_height = status.index;
		let shards = self.runtime.runtime_api().get_shards(block_id, self.peer_id)?;
		let block_num = self.backend.blockchain().number(block_id)?.unwrap();
		let block_num: i64 = block_num.to_string().parse()?;
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id)?;
			log::info!("got task ====== {:?}", tasks);
			for executable_task in tasks.iter().copied() {
				let task_id = executable_task.task_id;
				let cycle = executable_task.cycle;
				if self.running_tasks.contains(&(task_id, cycle)) {
					continue;
				}
				let task_descr = self.runtime.runtime_api().get_task(block_id, task_id)?.unwrap();
				if block_height >= task_descr.trigger(cycle) {
					log::info!("Running Task {:?}", task_id);
					self.running_tasks.insert(executable_task);
					let task = Task::new(self.sign_data_sender.clone(), wallet.clone());
					let storage = self.backend.offchain_storage().unwrap();
					tokio::task::spawn(async move {
						let result = task
							.execute(block_height, shard_id, task_id, cycle, task_descr, block_num)
							.await
							.map_err(|e| e.to_string());

						match result {
							Ok(signature) => {
								let status = ScheduleStatus { shard_id, signature };
								time_primitives::write_message(
									storage,
									&OcwPayload::SubmitTaskResult { task_id, cycle, status },
								);
							},
							Err(error) => {
								let error_status = ScheduleError { shard_id, error_string: error };
								time_primitives::write_message(
									storage,
									&OcwPayload::SubmitTaskError { task_id, error: error_status },
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
		let mut finality_notifications = self.runtime.finality_notification_stream();
		while let Some(notification) = finality_notifications.next().await {
			if let Err(err) = self.start_tasks(notification.header.hash()).await {
				log::error!("error processing tasks: {}", err);
			}
		}
	}
}
