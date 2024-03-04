use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
use rosetta_client::client::GenericClientStream;
use rosetta_client::Wallet;
use rosetta_config_ethereum::CallResult;
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use schnorr_evm::VerifyingKey;
use std::{
	future::Future,
	path::PathBuf,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use time_primitives::{
	BlockNumber, Function, Payload, Runtime, ShardId, TaskId, TaskResult, TssHash, TssSignature,
	TssSigningRequest,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct TaskSpawnerParams<S> {
	pub tss: mpsc::Sender<TssSigningRequest>,
	pub blockchain: String,
	pub network: String,
	pub url: String,
	pub keyfile: PathBuf,
	pub substrate: S,
}

#[derive(Clone)]
pub struct TaskSpawner<S> {
	tss: mpsc::Sender<TssSigningRequest>,
	wallet: Arc<Wallet>,
	wallet_guard: Arc<Mutex<()>>,
	substrate: S,
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
				None,
			)
			.await?,
		);
		Ok(Self {
			tss: params.tss,
			wallet,
			wallet_guard: Arc::new(Mutex::new(())),
			substrate: params.substrate,
		})
	}

	async fn execute_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<Payload> {
		Ok(match function {
			Function::EvmViewCall { address, input } => {
				let data = self
					.wallet
					.eth_view_call(*address, input.clone(), target_block_number.into())
					.await?;
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
				Payload::Hashed(VerifyingKey::message_hash(json.to_string().as_bytes()))
			},
			Function::EvmTxReceipt { tx } => {
				let Some(receipt) = self.wallet.eth_transaction_receipt(*tx).await? else {
					anyhow::bail!("transaction receipt from tx {} not found", hex::encode(tx));
				};
				let json = serde_json::json!({
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
				.to_string();
				Payload::Hashed(VerifyingKey::message_hash(json.to_string().as_bytes()))
			},
			Function::ReadMessages => Payload::Gmp(vec![]),
			_ => anyhow::bail!("not a read function {function:?}"),
		})
	}

	async fn tss_sign(
		&self,
		block_number: BlockNumber,
		shard_id: ShardId,
		task_id: TaskId,
		payload: &[u8],
	) -> Result<(TssHash, TssSignature)> {
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssSigningRequest {
				request_id: task_id,
				shard_id,
				block_number,
				data: payload.to_vec(),
				tx,
			})
			.await?;
		Ok(rx.await?)
	}

	#[allow(clippy::too_many_arguments)]
	async fn read(
		self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		function: Function,
		block_num: BlockNumber,
	) -> Result<()> {
		let result = self
			.execute_function(&function, target_block)
			.await
			.map_err(|err| format!("{err}"));
		let payload = match result {
			Ok(payload) => payload,
			Err(payload) => Payload::Error(payload),
		};
		let (_, signature) =
			self.tss_sign(block_num, shard_id, task_id, &payload.bytes(task_id)).await?;
		let result = TaskResult { shard_id, payload, signature };
		if let Err(e) = self.substrate.submit_task_result(task_id, result).await {
			tracing::error!("Error submitting task result {:?}", e);
		}
		Ok(())
	}

	async fn write(self, task_id: TaskId, function: Function) -> Result<()> {
		let tx_hash = match function {
			Function::EvmDeploy { bytecode } => {
				let _guard = self.wallet_guard.lock().await;
				self.wallet.eth_deploy_contract(bytecode.clone()).await?
			},
			Function::EvmCall { address, input, amount } => {
				let _guard = self.wallet_guard.lock().await;
				self.wallet.eth_send_call(address, input.clone(), amount, None, None).await?
			},
			_ => anyhow::bail!("not a write function {function:?}"),
		};
		if let Err(e) = self.substrate.submit_task_hash(task_id, tx_hash).await {
			tracing::error!("Error submitting task hash {:?}", e);
		}
		Ok(())
	}

	async fn sign(
		self,
		shard_id: ShardId,
		task_id: TaskId,
		payload: Vec<u8>,
		block_number: u32,
	) -> Result<()> {
		let (_, sig) = self.tss_sign(block_number, shard_id, task_id, &payload).await?;
		if let Err(e) = self.substrate.submit_task_signature(task_id, sig).await {
			tracing::error!("Error submitting task signature{:?}", e);
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

	fn execute_read(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		function: Function,
		block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().read(target_block, shard_id, task_id, function, block_num).boxed()
	}

	fn execute_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		payload: Vec<u8>,
		block_num: u32,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().sign(shard_id, task_id, payload, block_num).boxed()
	}

	fn execute_write(
		&self,
		task_id: TaskId,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().write(task_id, function).boxed()
	}
}

#[allow(clippy::type_complexity)]
struct BlockStream<'a> {
	wallet: &'a Arc<Wallet>,
	opening:
		Option<Pin<Box<dyn Future<Output = Result<Option<GenericClientStream<'a>>>> + Send + 'a>>>,
	listener: Option<GenericClientStream<'a>>,
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
						ClientEvent::Event(_) => continue,
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
