use anyhow::{anyhow, Context as _, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
use rosetta_client::{types::PartialBlockIdentifier, Blockchain, Wallet};
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use serde_json::Value;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{
	future::Future,
	marker::PhantomData,
	path::Path,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use time_primitives::{
	Function, Network, ShardId, SubmitTasks, TaskCycle, TaskError, TaskId, TaskResult, TasksApi,
	TssHash, TssId, TssSignature, TssSigningRequest,
};
use timegraph_client::{Timegraph, TimegraphData};

pub struct TaskSpawnerParams<B: Block, R, TxSub> {
	pub _marker: PhantomData<B>,
	pub tss: mpsc::Sender<TssSigningRequest>,
	pub blockchain: Network,
	pub network: String,
	pub url: String,
	pub keyfile: Option<String>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
	pub runtime: Arc<R>,
	pub tx_submitter: TxSub,
}

impl<B: Block, R, TxSub: Clone> Clone for TaskSpawnerParams<B, R, TxSub> {
	fn clone(&self) -> Self {
		Self {
			_marker: self._marker,
			tss: self.tss.clone(),
			blockchain: self.blockchain,
			network: self.network.clone(),
			url: self.url.clone(),
			keyfile: self.keyfile.clone(),
			timegraph_url: self.timegraph_url.clone(),
			timegraph_ssk: self.timegraph_ssk.clone(),
			runtime: self.runtime.clone(),
			tx_submitter: self.tx_submitter.clone(),
		}
	}
}

pub struct TaskSpawner<B, R, TxSub> {
	_marker: PhantomData<B>,
	tss: mpsc::Sender<TssSigningRequest>,
	wallet: Arc<Wallet>,
	timegraph: Option<Arc<Timegraph>>,
	runtime: Arc<R>,
	tx_submitter: TxSub,
}

impl<B, R, TxSub> Clone for TaskSpawner<B, R, TxSub>
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

impl<B, R, TxSub> TaskSpawner<B, R, TxSub>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	TxSub: SubmitTasks + Clone + Send + Sync + 'static,
{
	pub async fn new(params: TaskSpawnerParams<B, R, TxSub>) -> Result<Self> {
		let path = params.keyfile.as_ref().map(Path::new);
		let blockchain = match params.blockchain {
			Network::Ethereum => Blockchain::Ethereum,
			Network::Astar => Blockchain::Astar,
			Network::Polygon => Blockchain::Polygon,
		};
		let wallet = Arc::new(Wallet::new(blockchain, &params.network, &params.url, path).await?);
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
	) -> Result<Vec<u8>> {
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
				serde_json::to_string(&data)?.into_bytes()
			},
			Function::EvmTxReceipt { tx } => {
				let data = self.wallet.eth_transaction_receipt(tx).await?;
				serde_json::to_string(&data)?.into_bytes()
			},
			Function::EvmDeploy { bytecode } => {
				self.wallet.eth_deploy_contract(bytecode.clone()).await?
			},
			Function::EvmCall {
				address,
				function_signature,
				input,
				amount,
			} => self.wallet.eth_send_call(address, function_signature, input, *amount).await?,
		})
	}

	async fn tss_sign(
		&self,
		block_number: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		payload: &[u8],
	) -> Result<(TssHash, TssSignature)> {
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssSigningRequest {
				request_id: TssId(task_id, cycle),
				shard_id,
				block_number,
				data: payload.to_vec(),
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
		payload: &[u8],
		signature: TssSignature,
	) -> Result<()> {
		if let Some(timegraph) = self.timegraph.as_ref() {
			if matches!(function, Function::EvmViewCall { .. }) {
				let result_json = serde_json::from_slice(payload)?;
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
			Ok(payload) => payload.as_slice(),
			Err(payload) => payload.as_bytes(),
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
					tracing::error!("Error submitting task result {:?}", e);
				}
			},
			Err(msg) => {
				let error = TaskError { shard_id, msg, signature };
				if let Err(e) = self.tx_submitter.submit_task_error(task_id, task_cycle, error) {
					tracing::error!("Error submitting task error {:?}", e);
				}
			},
		}
		Ok(())
	}

	async fn write(self, task_id: TaskId, cycle: TaskCycle, function: Function) -> Result<()> {
		let tx_hash = self.execute_function(&function, 0).await?;
		if let Err(e) = self.tx_submitter.submit_task_hash(task_id, cycle, tx_hash) {
			tracing::error!("Error submitting task hash {:?}", e);
		}
		Ok(())
	}
}

impl<B, R, TxSub> super::TaskSpawner for TaskSpawner<B, R, TxSub>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	TxSub: SubmitTasks + Clone + Send + Sync + 'static,
{
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		Box::pin(BlockStream::new(&self.wallet))
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

struct BlockStream<'a> {
	wallet: &'a Arc<Wallet>,
	opening: Option<
		Pin<
			Box<
				dyn Future<
						Output = Result<
							Option<Pin<Box<dyn Stream<Item = ClientEvent> + Send + Unpin + 'a>>>,
						>,
					> + Send
					+ 'a,
			>,
		>,
	>,
	listener: Option<Pin<Box<dyn Stream<Item = ClientEvent> + Send + Unpin + 'a>>>,
}

impl<'a> BlockStream<'a> {
	pub fn new(wallet: &'a Arc<Wallet>) -> Self {
		Self {
			wallet,
			opening: None,
			listener: None,
		}
	}
}

impl<'a> Stream for BlockStream<'a> {
	type Item = u64;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		loop {
			if let Some(listener) = self.listener.as_mut() {
				match Pin::new(listener).poll_next(cx) {
					Poll::Ready(Some(event)) => match event {
						ClientEvent::NewFinalized(BlockOrIdentifier::Identifier(identifier)) => {
							return Poll::Ready(Some(identifier.index));
						},
						ClientEvent::NewFinalized(BlockOrIdentifier::Block(block)) => {
							return Poll::Ready(Some(block.block_identifier.index));
						},
						ClientEvent::NewHead(_) => continue,
						ClientEvent::Close(reason) => {
							tracing::info!("block stream closed {}", reason);
							self.listener.take();
						},
					},
					Poll::Ready(None) => {
						self.listener.take();
					},
					Poll::Pending => return Poll::Pending,
				}
			}
			if let Some(opening) = self.opening.as_mut() {
				match Pin::new(opening).poll(cx) {
					Poll::Ready(Ok(Some(stream))) => {
						self.opening.take();
						self.listener = Some(stream);
					},
					Poll::Ready(Ok(None)) => {
						self.opening.take();
						tracing::info!("error opening listener");
					},
					Poll::Ready(Err(err)) => {
						self.opening.take();
						tracing::info!("error opening listener {}", err);
					},
					Poll::Pending => return Poll::Pending,
				}
			}
			let wallet = self.wallet;
			self.opening = Some(wallet.listen().boxed());
		}
	}
}
