use anyhow::{anyhow, Context as _, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
use rosetta_client::{types::PartialBlockIdentifier, Blockchain, Wallet};
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use schnorr_evm::VerifyingKey;
use serde_json::Value;
use std::{
	future::Future,
	path::PathBuf,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use time_primitives::{
	append_hash_with_task_data, BlockNumber, Function, Network, ShardId, TaskCycle, TaskError,
	TaskId, TaskResult, Tasks, TssHash, TssId, TssSignature, TssSigningRequest,
};
use timegraph_client::{Timegraph, TimegraphData};

#[derive(Clone)]
pub struct TaskSpawnerParams<S> {
	pub tss: mpsc::Sender<TssSigningRequest>,
	pub blockchain: Network,
	pub network: String,
	pub url: String,
	pub keyfile: Option<PathBuf>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
	pub substrate: S,
}

#[derive(Clone)]
pub struct TaskSpawner<S> {
	tss: mpsc::Sender<TssSigningRequest>,
	wallet: Arc<Wallet>,
	timegraph: Option<Arc<Timegraph>>,
	substrate: S,
}

impl<S> TaskSpawner<S>
where
	S: Tasks + Send,
{
	pub async fn new(params: TaskSpawnerParams<S>) -> Result<Self> {
		let blockchain = match params.blockchain {
			Network::Ethereum => Blockchain::Ethereum,
			Network::Astar => Blockchain::Astar,
			Network::Polygon => Blockchain::Polygon,
		};
		let wallet = Arc::new(
			Wallet::new(blockchain, &params.network, &params.url, params.keyfile.as_deref())
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
			substrate: params.substrate,
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
			Function::SendMessage { .. }
			| Function::RegisterKeys { .. }
			| Function::UnregisterKeys { .. } => {
				return Err(anyhow!(
					"SendMessage must be transformed into EvmCall prior to execution"
				))
			},
		})
	}

	async fn tss_sign(
		&self,
		block_number: BlockNumber,
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
		block_num: BlockNumber,
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
						timechain_block_number: block_num as _,
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
		block_num: BlockNumber,
	) -> Result<()> {
		let result = self
			.execute_function(&function, target_block)
			.await
			.map_err(|err| format!("{:?}", err));
		let payload = match &result {
			Ok(payload) => payload.as_slice(),
			Err(payload) => payload.as_bytes(),
		};
		let prehashed_payload = VerifyingKey::message_hash(payload);
		let hash = append_hash_with_task_data(prehashed_payload, task_id, task_cycle);
		let (_, signature) = self.tss_sign(block_num, shard_id, task_id, task_cycle, &hash).await?;
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
				let result = TaskResult {
					shard_id,
					hash: prehashed_payload,
					signature,
				};
				if let Err(e) = self.substrate.submit_task_result(task_id, task_cycle, result) {
					tracing::error!("Error submitting task result {:?}", e);
				}
			},
			Err(msg) => {
				let error = TaskError { shard_id, msg, signature };
				if let Err(e) = self.substrate.submit_task_error(task_id, task_cycle, error) {
					tracing::error!("Error submitting task error {:?}", e);
				}
			},
		}
		Ok(())
	}

	async fn sign(
		self,
		shard_id: ShardId,
		task_id: TaskId,
		task_cycle: TaskCycle,
		payload: Vec<u8>,
		block_number: u32,
	) -> Result<()> {
		let (_, sig) = self.tss_sign(block_number, shard_id, task_id, task_cycle, &payload).await?;
		if let Err(e) = self.substrate.submit_task_signature(task_id, sig) {
			tracing::error!("Error submitting task signature{:?}", e);
		}
		Ok(())
	}

	async fn write(self, task_id: TaskId, cycle: TaskCycle, function: Function) -> Result<()> {
		let tx_hash = self.execute_function(&function, 0).await?;
		if let Err(e) = self.substrate.submit_task_hash(task_id, cycle, tx_hash) {
			tracing::error!("Error submitting task hash {:?}", e);
		}
		Ok(())
	}
}

impl<S> super::TaskSpawner for TaskSpawner<S>
where
	S: Tasks + Clone + Send + Sync + 'static,
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
		block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone()
			.read(target_block, shard_id, task_id, cycle, function, collection, block_num)
			.boxed()
	}

	fn execute_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		payload: Vec<u8>,
		block_num: u32,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().sign(shard_id, task_id, cycle, payload, block_num).boxed()
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

#[allow(clippy::type_complexity)]
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
