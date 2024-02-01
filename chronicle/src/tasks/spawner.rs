use anyhow::{anyhow, Context as _, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
use rosetta_client::Wallet;
use rosetta_config_ethereum::{AtBlock, CallResult};
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
	append_hash_with_task_data, BlockNumber, Function, Runtime, ShardId, TaskCycle, TaskError,
	TaskId, TaskResult, TssHash, TssId, TssSignature, TssSigningRequest,
};
use timegraph_client::{Timegraph, TimegraphData};

#[derive(Clone)]
pub struct TaskSpawnerParams<S> {
	pub tss: mpsc::Sender<TssSigningRequest>,
	pub blockchain: String,
	pub network: String,
	pub url: String,
	pub keyfile: PathBuf,
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
	chain_id: u64,
}

impl<S> TaskSpawner<S>
where
	S: Runtime,
{
	pub async fn new(params: TaskSpawnerParams<S>) -> Result<Self> {
		let wallet = Arc::new(
			Wallet::new(
				params.blockchain.parse()?,
				&params.network,
				&params.url,
				Some(&params.keyfile),
			)
			.await?,
		);
		let chain_id = wallet.eth_chain_id().await?;
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
			chain_id,
		})
	}

	async fn execute_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<Vec<u8>> {
		let block = AtBlock::Number(target_block_number);
		Ok(match function {
			Function::EvmViewCall { address, input } => {
				let data = self.wallet.eth_view_call(*address, input.clone(), block).await?;
				let json = match data {
					// Call executed successfully
					CallResult::Success(data) => serde_json::json!({
						"success": hex::encode(data),
					}),
					// Call reverted
					CallResult::Revert(data) => serde_json::json!({
						"revert": hex::encode(data),
					}),
					// Call invalid or EVM error
					CallResult::Error => serde_json::json!({
						"error": null,
					}),
				};
				json.to_string().into_bytes()
			},
			Function::EvmTxReceipt { tx } => {
				if tx.len() != 32 {
					anyhow::bail!("invalid transaction hash length, expected 32 got {}", tx.len());
				}
				let mut tx_hash = [0u8; 32];
				tx_hash.copy_from_slice(tx.as_ref());
				let Some(receipt) = self.wallet.eth_transaction_receipt(tx_hash).await? else {
					anyhow::bail!("transaction receipt from tx {} not found", hex::encode(tx_hash));
				};
				serde_json::json!({
					"transactionHash": format!("{:?}", receipt.transaction_hash),
					"transactionIndex": receipt.transaction_index,
					"blockHash": receipt.block_hash.map(|block_hash| format!("{block_hash:?}")),
					"blockNumber": receipt.block_number,
					"from": format!("{:?}", receipt.from),
					"to": receipt.to.map(|to| format!("{to:?}")),
					"gasUsed": receipt.gas_used.map(|gas_used| format!("{gas_used:?}")),
					"contractAddress": receipt.contract_address.map(|contract| format!("{contract:?}")),
					"status": receipt.status_code,
					"type": receipt.transaction_type,
				})
				.to_string()
				.into_bytes()
			},
			Function::EvmDeploy { bytecode } => {
				self.wallet.eth_deploy_contract(bytecode.clone()).await?.to_vec()
			},
			Function::EvmCall { address, input, amount } => {
				self.wallet.eth_send_call(*address, input.clone(), *amount).await?.to_vec()
			},
			Function::RegisterShard { .. } => {
				return Err(anyhow!(
					"RegisterShard must be transformed into EvmCall prior to execution"
				));
			},
			Function::UnregisterShard { .. } => {
				return Err(anyhow!(
					"UnregisterShard must be transformed into EvmCall prior to execution"
				));
			},
			Function::SendMessage { .. } => {
				return Err(anyhow!(
					"SendMessage must be transformed into EvmCall prior to execution"
				));
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
		collection: Option<[u8; 32]>,
		block_num: BlockNumber,
		payload: &[u8],
		signature: TssSignature,
	) -> Result<()> {
		if let (Some(timegraph), Some(collection)) = (self.timegraph.as_ref(), collection) {
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
						collection: hex::encode(collection),
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
		collection: Option<[u8; 32]>,
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
				if let Err(e) = self
					.submit_timegraph(
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
					.await
				{
					tracing::error!("Error submitting to timegraph {:?}", e);
				}
				let result = TaskResult {
					shard_id,
					hash: prehashed_payload,
					signature,
				};
				if let Err(e) = self.substrate.submit_task_result(task_id, task_cycle, result).await
				{
					tracing::error!("Error submitting task result {:?}", e);
				}
			},
			Err(msg) => {
				let error = TaskError { shard_id, msg, signature };
				if let Err(e) = self.substrate.submit_task_error(task_id, task_cycle, error).await {
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
		if let Err(e) = self.substrate.submit_task_signature(task_id, sig).await {
			tracing::error!("Error submitting task signature{:?}", e);
		}
		Ok(())
	}

	async fn write(self, task_id: TaskId, cycle: TaskCycle, function: Function) -> Result<()> {
		let tx_hash = self.execute_function(&function, 0).await?;
		if let Err(e) = self.substrate.submit_task_hash(task_id, cycle, tx_hash).await {
			tracing::error!("Error submitting task hash {:?}", e);
		}
		Ok(())
	}
}

impl<S> super::TaskSpawner for TaskSpawner<S>
where
	S: Runtime,
{
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		Box::pin(BlockStream::new(&self.wallet))
	}

	fn chain_id(&self) -> u64 {
		self.chain_id
	}

	fn execute_read(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		function: Function,
		collection: Option<[u8; 32]>,
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
		task_id: TaskId,
		cycle: TaskCycle,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().write(task_id, cycle, function).boxed()
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
