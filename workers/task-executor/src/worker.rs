use crate::{TaskExecutorParams, TW_LOG};
use anyhow::{anyhow, Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt};
use rosetta_client::{create_wallet, types::PartialBlockIdentifier, EthereumExt, Wallet};
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use serde_json::Value;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_runtime::traits::Block;
use std::marker::{Send, Sync};
use std::{
	collections::BTreeSet, future::Future, marker::PhantomData, path::Path, pin::Pin, sync::Arc,
};
use time_primitives::{
	Function, PublicKey, ShardId, TaskCycle, TaskError, TaskExecution, TaskId, TaskResult,
	TaskSpawner, TasksApi, TssId, TssRequest, TssSignature,
};
use timegraph_client::{Timegraph, TimegraphData};

pub struct TaskSpawnerParams<B: Block, C, R> {
	pub tss: mpsc::Sender<TssRequest>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
	pub keyfile: Option<String>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub kv: KeystorePtr,
	pub offchain_tx_pool_factory: OffchainTransactionPoolFactory<B>,
}

pub struct Task<B: Block, C, R> {
	tss: mpsc::Sender<TssRequest>,
	wallet: Arc<Wallet>,
	timegraph: Option<Arc<Timegraph>>,
	client: Arc<C>,
	runtime: Arc<R>,
	kv: KeystorePtr,
	offchain_tx_pool_factory: OffchainTransactionPoolFactory<B>,
}

impl<B: Block, C, R> Clone for Task<B, C, R> {
	fn clone(&self) -> Self {
		Self {
			tss: self.tss.clone(),
			wallet: self.wallet.clone(),
			timegraph: self.timegraph.clone(),
			client: self.client.clone(),
			runtime: self.runtime.clone(),
			kv: self.kv.clone(),
			offchain_tx_pool_factory: self.offchain_tx_pool_factory.clone(),
		}
	}
}

impl<B, C, R> Task<B, C, R>
where
	B: Block,
	C: HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
{
	pub async fn new(params: TaskSpawnerParams<B, C, R>) -> Result<Self> {
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
			tss: params.tss,
			wallet,
			timegraph,
			client: params.client,
			runtime: params.runtime,
			kv: params.kv,
			offchain_tx_pool_factory: params.offchain_tx_pool_factory,
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
	) -> Result<([u8; 32], TssSignature)> {
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssRequest {
				request_id: TssId(task_id, cycle),
				shard_id,
				block_number,
				data: payload.as_bytes().to_vec(),
				tx,
			})
			.await?;
		Ok(rx.await?)
	}

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
			self.tss_sign(block_num, shard_id, task_id, task_cycle, &payload).await?;
		let block_hash = self.client.info().best_hash;
		let mut api = self.runtime.runtime_api();
		api.register_extension(KeystoreExt(self.kv.clone()));
		api.register_extension(self.offchain_tx_pool_factory.offchain_transaction_pool(block_hash));
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
				api.submit_task_result(block_hash, task_id, task_cycle, result)?;
			},
			Err(msg) => {
				let error = TaskError { shard_id, msg, signature };
				api.submit_task_error(block_hash, task_id, task_cycle, error)?;
			},
		}
		Ok(())
	}

	async fn write(self, shard_id: ShardId, task_id: TaskId, function: Function) -> Result<()> {
		let tx_hash = self.execute_function(&function, 0).await?;
		let block_hash = self.client.info().best_hash;
		let mut api = self.runtime.runtime_api();
		api.register_extension(KeystoreExt(self.kv.clone()));
		api.register_extension(self.offchain_tx_pool_factory.offchain_transaction_pool(block_hash));
		api.submit_task_hash(block_hash, shard_id, task_id, tx_hash)?;
		Ok(())
	}
}

#[async_trait::async_trait]
impl<B, C, R> TaskSpawner for Task<B, C, R>
where
	B: Block,
	C: HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
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

pub struct TaskExecutor<B: Block, R, T> {
	_block: PhantomData<B>,
	runtime: Arc<R>,
	public_key: PublicKey,
	running_tasks: BTreeSet<TaskExecution>,
	task_spawner: T,
}

impl<B, R, T> TaskExecutor<B, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	T: TaskSpawner,
{
	pub fn new(params: TaskExecutorParams<B, R, T>) -> Self {
		let TaskExecutorParams {
			_block,
			runtime,
			network: _,
			public_key,
			task_spawner,
		} = params;
		Self {
			_block,
			runtime,
			public_key,
			running_tasks: Default::default(),
			task_spawner,
		}
	}

	pub async fn start_tasks(
		&mut self,
		block_hash: <B as Block>::Hash,
		block_num: u64,
		shard_id: ShardId,
	) -> Result<()> {
		let block_height =
			self.task_spawner.block_height().await.context("Failed to fetch block height")?;
		let tasks = self.runtime.runtime_api().get_shard_tasks(block_hash, shard_id)?;
		log::info!(target: TW_LOG, "got task ====== {:?}", tasks);
		for executable_task in tasks.iter().clone() {
			let task_id = executable_task.task_id;
			let cycle = executable_task.cycle;
			let retry_count = executable_task.retry_count;
			if self.running_tasks.contains(executable_task) {
				continue;
			}
			let task_descr = self.runtime.runtime_api().get_task(block_hash, task_id)?.unwrap();
			let target_block_number = task_descr.trigger(cycle);
			let function = task_descr.function;
			let hash = task_descr.hash;
			if block_height >= target_block_number {
				log::info!(target: TW_LOG, "Running Task {}, {:?}", executable_task, executable_task.phase);
				self.running_tasks.insert(executable_task.clone());
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
				tokio::task::spawn(async move {
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
			}
		}
		self.running_tasks.retain(|x| tasks.contains(x));
		Ok(())
	}
}
