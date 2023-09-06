use crate::TW_LOG;
use anyhow::{anyhow, Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt};
use rosetta_client::{create_wallet, types::PartialBlockIdentifier, EthereumExt, Wallet};
use serde_json::Value;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{
	collections::BTreeMap, future::Future, marker::PhantomData, path::Path, pin::Pin, sync::Arc,
	time::Duration,
};
use time_primitives::{
	Function, Network, PublicKey, ShardId, SubmitTasks, TaskCycle, TaskError, TaskExecution,
	TaskId, TaskResult, TaskSpawner, TasksApi, TssHash, TssId, TssSignature, TssSigningRequest,
};
use timegraph_client::{Timegraph, TimegraphData};
use tokio::task::JoinHandle;

pub struct TaskSpawnerParams<B: Block, R, TxSub> {
	pub _marker: PhantomData<B>,
	pub tss: mpsc::Sender<TssSigningRequest>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
	pub keyfile: Option<String>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
	pub runtime: Arc<R>,
	pub tx_submitter: TxSub,
}

pub struct Task<B, R, TxSub> {
	_marker: PhantomData<B>,
	tss: mpsc::Sender<TssSigningRequest>,
	wallet: Arc<Wallet>,
	timegraph: Option<Arc<Timegraph>>,
	runtime: Arc<R>,
	tx_submitter: TxSub,
}

impl<B, R, TxSub> Clone for Task<B, R, TxSub>
where
	B: Block,
	TxSub: SubmitTasks + Clone + Send + Sync + 'static,
{
	fn clone(&self) -> Self {
		Self {
			_marker: PhantomData,
			tss: self.tss.clone(),
			wallet: self.wallet.clone(),
			timegraph: self.timegraph.clone(),
			runtime: self.runtime.clone(),
			tx_submitter: self.tx_submitter.clone(),
		}
	}
}

impl<B, R, TxSub> Task<B, R, TxSub>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	TxSub: SubmitTasks + Clone + Send + Sync + 'static,
{
	pub async fn new(params: TaskSpawnerParams<B, R, TxSub>) -> Result<Self> {
		let path = params.keyfile.as_ref().map(Path::new);
		let wallet = Arc::new(
			create_wallet(
				params.connector_blockchain,
				params.connector_network,
				params.connector_url,
				path,
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
			_marker: PhantomData,
			tss: params.tss,
			wallet,
			timegraph,
			runtime: params.runtime,
			tx_submitter: params.tx_submitter,
		})
	}

	async fn execute_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<String> {
		let block = PartialBlockIdentifier {
			index: Some(target_block_number),
			hash: None,
		};
		Ok(match function {
			Function::EvmViewCall {
				address,
				function_signature,
				input,
			} => {
				let data = self
					.wallet
					.eth_view_call(address, function_signature, input, Some(block))
					.await?;
				serde_json::to_string(&data.result)?
			},
			Function::EvmTxReceipt { tx } => {
				let data = self.wallet.eth_transaction_receipt(tx).await?;
				serde_json::to_string(&data.result)?
			},
			Function::EvmDeploy { bytecode } => {
				self.wallet.eth_deploy_contract(bytecode.clone()).await?.hash
			},
			Function::EvmCall {
				address,
				function_signature,
				input,
				amount,
			} => {
				self.wallet
					.eth_send_call(address, function_signature, input, *amount)
					.await?
					.hash
			},
		})
	}

	async fn tss_sign(
		&self,
		block_number: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		payload: &str,
	) -> Result<(TssHash, TssSignature)> {
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssSigningRequest {
				request_id: TssId(task_id, cycle),
				shard_id,
				block_number,
				data: payload.as_bytes().to_vec(),
				tx,
			})
			.await?;
		Ok(rx.await?)
	}

	#[allow(clippy::too_many_arguments)]
	async fn submit_timegraph(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		task_cycle: TaskCycle,
		function: &Function,
		collection: String,
		block_num: u64,
		payload: &str,
		signature: TssSignature,
	) -> Result<()> {
		if let Some(timegraph) = self.timegraph.as_ref() {
			if matches!(function, Function::EvmViewCall { .. }) {
				let result_json = serde_json::from_str(payload)?;
				let formatted_result = match result_json {
					Value::Array(val) => val
						.iter()
						.filter_map(|x| x.as_str())
						.map(|x| x.to_string())
						.collect::<Vec<String>>(),
					v => vec![v.to_string()],
				};
				timegraph
					.submit_data(TimegraphData {
						collection,
						task_id,
						task_cycle,
						target_block_number: target_block,
						timechain_block_number: block_num,
						shard_id,
						signature,
						data: formatted_result,
					})
					.await
					.context("Failed to submit data to timegraph")?;
			}
		}
		Ok(())
	}

	#[allow(clippy::too_many_arguments)]
	async fn read(
		self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		task_cycle: TaskCycle,
		function: Function,
		collection: String,
		block_num: u64,
	) -> Result<()> {
		let result = self
			.execute_function(&function, target_block)
			.await
			.map_err(|err| format!("{:?}", err));
		let payload = match &result {
			Ok(payload) => payload.as_str(),
			Err(payload) => payload.as_str(),
		};
		let (hash, signature) =
			self.tss_sign(block_num, shard_id, task_id, task_cycle, payload).await?;
		match result {
			Ok(result) => {
				self.submit_timegraph(
					target_block,
					shard_id,
					task_id,
					task_cycle,
					&function,
					collection,
					block_num,
					&result,
					signature,
				)
				.await?;
				let result = TaskResult { shard_id, hash, signature };
				if let Err(e) = self.tx_submitter.submit_task_result(task_id, task_cycle, result) {
					log::error!("Error submitting task result {:?}", e);
				}
			},
			Err(msg) => {
				let error = TaskError { shard_id, msg, signature };
				if let Err(e) = self.tx_submitter.submit_task_error(task_id, task_cycle, error) {
					log::error!("Error submitting task error {:?}", e);
				}
			},
		}
		Ok(())
	}

	async fn write(self, shard_id: ShardId, task_id: TaskId, function: Function) -> Result<()> {
		let tx_hash = self.execute_function(&function, 0).await?;
		if let Err(e) = self.tx_submitter.submit_task_hash(shard_id, task_id, tx_hash) {
			log::error!("Error submitting task hash {:?}", e);
		}
		Ok(())
	}
}

#[async_trait::async_trait]
impl<B, R, TxSub> TaskSpawner for Task<B, R, TxSub>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	TxSub: SubmitTasks + Clone + Send + Sync + 'static,
{
	async fn block_height(&self) -> Result<u64> {
		let status = self.wallet.status().await?;
		Ok(status.index)
	}

	fn execute_read(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		function: Function,
		collection: String,
		block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone()
			.read(target_block, shard_id, task_id, cycle, function, collection, block_num)
			.boxed()
	}

	fn execute_write(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().write(shard_id, task_id, function).boxed()
	}
}

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<B: Block, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	R::Api: TasksApi<B>,
	T: TaskSpawner + Send + Sync + 'static,
{
	pub _block: PhantomData<B>,
	pub runtime: Arc<R>,
	pub task_spawner: T,
	pub network: Network,
	pub public_key: PublicKey,
}

pub struct TaskExecutor<B: Block, R, T> {
	_block: PhantomData<B>,
	runtime: Arc<R>,
	task_spawner: T,
	network: Network,
	public_key: PublicKey,
	running_tasks: BTreeMap<TaskExecution<u32>, JoinHandle<()>>,
	block_height: u64,
}

#[async_trait::async_trait]
impl<B, R, T> time_primitives::TaskExecutor<B> for TaskExecutor<B, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	T: TaskSpawner + Send + Sync + 'static,
{
	fn network(&self) -> Network {
		self.network()
	}

	async fn poll_block_height(&mut self) {
		self.poll_block_height().await
	}

	fn start_tasks(
		&mut self,
		block_hash: <B as Block>::Hash,
		block_num: u64,
		shard_id: ShardId,
	) -> Result<()> {
		self.start_tasks(block_hash, block_num, shard_id)
	}
}

impl<B, R, T> TaskExecutor<B, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	T: TaskSpawner + Send + Sync + 'static,
{
	pub fn new(params: TaskExecutorParams<B, R, T>) -> Self {
		let TaskExecutorParams {
			_block,
			runtime,
			task_spawner,
			network,
			public_key,
		} = params;
		Self {
			_block,
			runtime,
			task_spawner,
			network,
			public_key,
			block_height: 0,
			running_tasks: Default::default(),
		}
	}

	pub fn network(&self) -> Network {
		self.network
	}

	pub async fn poll_block_height(&mut self) {
		match self.task_spawner.block_height().await {
			Ok(block_height) => {
				self.block_height = block_height;
			},
			Err(error) => {
				log::error!(target: TW_LOG, "failed to fetch block height: {:?}", error);
			},
		}
		tokio::time::sleep(Duration::from_secs(10)).await;
	}
	pub fn start_tasks(
		&mut self,
		block_hash: <B as Block>::Hash,
		block_num: u64,
		shard_id: ShardId,
	) -> Result<()> {
		let tasks = self.runtime.runtime_api().get_shard_tasks(block_hash, shard_id)?;
		log::info!(target: TW_LOG, "got task ====== {:?}", tasks);
		for executable_task in tasks.iter().clone() {
			let task_id = executable_task.task_id;
			let cycle = executable_task.cycle;
			let retry_count = executable_task.retry_count;
			if self.running_tasks.contains_key(executable_task) {
				continue;
			}
			let task_descr = self.runtime.runtime_api().get_task(block_hash, task_id)?.unwrap();
			let target_block_number = task_descr.trigger(cycle);
			let function = task_descr.function;
			let hash = task_descr.hash;
			if self.block_height >= target_block_number {
				log::info!(target: TW_LOG, "Running Task {}, {:?}", executable_task, executable_task.phase);
				let task = if let Some(public_key) = executable_task.phase.public_key() {
					if *public_key != self.public_key {
						log::info!(target: TW_LOG, "Skipping task {} due to public_key mismatch", task_id);
						continue;
					}
					self.task_spawner.execute_write(shard_id, task_id, function)
				} else {
					let function = if let Some(tx) = executable_task.phase.tx_hash() {
						Function::EvmTxReceipt { tx: tx.to_string() }
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
						block_num,
					)
				};
				let handle = tokio::task::spawn(async move {
					match task.await {
						Ok(()) => {
							log::info!(
								target: TW_LOG,
								"Task {}/{}/{} completed",
								task_id,
								cycle,
								retry_count,
							);
						},
						Err(error) => {
							log::error!(
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
			}
		}
		self.running_tasks.retain(|x, handle| {
			if tasks.contains(x) {
				true
			} else {
				if !handle.is_finished() {
					log::info!(target: TW_LOG, "Task {}/{}/{} aborted", x.task_id, x.cycle, x.retry_count);
					handle.abort();
				}
				false
			}
		});
		Ok(())
	}
}
