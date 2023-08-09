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
	Function, FunctionResult, OcwPayload, PeerId, ScheduleCycle, ScheduleStatus, ShardId, TaskId,
	TaskSchedule, TimeApi, TssRequest, TssSignature,
};

pub struct Task {
	tss: mpsc::Sender<TssRequest>,
	wallet: Arc<Wallet>,
}

impl Task {
	pub fn new(tss: mpsc::Sender<TssRequest>, wallet: Arc<Wallet>) -> Self {
		Self { tss, wallet }
	}

	/// Fetches and executes contract call for a given schedule_id
	async fn execute_function(&self, function: &Function) -> Result<FunctionResult> {
		match function {
			// If the task function is an Ethereum contract
			// call, call it and send for signing
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

	/// Encode call response and send data for tss signing process
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
		timechain_integration::submit_to_timegraph(target_block, &result, task.hash.clone(), block_num)
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
	wallet: Arc<Wallet>,
	running_tasks: HashSet<(TaskId, ScheduleCycle)>,
}

impl<B, BE, R> TaskExecutor<B, BE, R>
where
	B: Block,
	BE: Backend<B> + 'static,
	R: BlockchainEvents<B> + ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
{
	pub async fn new(params: TaskExecutorParams<B, BE, R>) -> Result<Self> {
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
		// create rosetta client and get chain configuration
		let wallet =
			create_wallet(connector_blockchain, connector_network, connector_url, None).await?;
		Ok(Self {
			_block,
			backend,
			runtime,
			peer_id,
			sign_data_sender,
			wallet: Arc::new(wallet),
			running_tasks: Default::default(),
		})
	}

	async fn start_tasks(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let status = self.wallet.status().await?;
		let block_height = status.index;
		let shards = self.runtime.runtime_api().get_shards(block_id, self.peer_id).unwrap();
		let block_num: i64 = if let Ok(Some(val)) = self.backend.blockchain().number(block_id) {
			//should not fail since we have u32 as BlockNumer
			val.to_string().parse().unwrap()
		}else{
			log::warn!("Something bad happened fetching block number");
			0	
		};
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id).unwrap();
			for (task_id, cycle) in tasks.clone() {
				if self.running_tasks.contains(&(task_id, cycle)) {
					continue;
				}
				let task_descr =
					self.runtime.runtime_api().get_task(block_id, task_id).unwrap().unwrap();
				if block_height >= task_descr.trigger(cycle) {
					self.running_tasks.insert((task_id, cycle));
					let task = Task::new(self.sign_data_sender.clone(), self.wallet.clone());
					let storage = self.backend.offchain_storage().unwrap();
					tokio::task::spawn(async move {
						let result = task
							.execute(block_height, shard_id, task_id, cycle, task_descr, block_num)
							.await
							.map_err(|e| e.to_string());
						let status = ScheduleStatus { shard_id, result };
						time_primitives::write_message(
							storage,
							&OcwPayload::SubmitTaskResult { task_id, cycle, status },
						);
					});
				}
			}
			for task_ran in self.running_tasks.clone() {
				if !tasks.contains(&task_ran) {
					self.running_tasks.remove(&task_ran);
				}
			}
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
