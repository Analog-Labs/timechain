use alloy_primitives::B256;
use alloy_sol_types::SolEvent;
use anyhow::{Context as _, Result};
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
use rosetta_client::client::GenericClientStream;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{query::GetLogs, CallResult, FilterBlockOption};
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use schnorr_evm::VerifyingKey;
use std::time::Duration;
use std::{
	future::Future,
	num::NonZeroU64,
	path::PathBuf,
	pin::Pin,
	sync::Arc,
	task::{Context, Poll},
};
use time_primitives::{
	BlockNumber, Function, NetworkId, Payload, Runtime, ShardId, TaskExecution, TaskId, TaskPhase,
	TaskResult, TssHash, TssSignature, TssSigningRequest,
};
use time_primitives::{IGateway, Msg};
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Clone)]
pub struct TaskSpawnerParams<S> {
	pub tss: mpsc::Sender<TssSigningRequest>,
	pub blockchain: String,
	pub min_balance: u128,
	pub network: String,
	pub network_id: NetworkId,
	pub url: String,
	pub keyfile: PathBuf,
	pub substrate: S,
}

#[derive(Clone)]
pub struct TaskSpawner<S> {
	tss: mpsc::Sender<TssSigningRequest>,
	wallet: Arc<Wallet>,
	wallet_min_balance: u128,
	wallet_guard: Arc<Mutex<()>>,
	substrate: S,
	network_id: NetworkId,
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

		let spawner = Self {
			tss: params.tss,
			wallet_min_balance: params.min_balance,
			wallet,
			wallet_guard: Arc::new(Mutex::new(())),
			substrate: params.substrate,
			network_id: params.network_id,
		};

		while !spawner.is_balance_available().await? {
			sleep(Duration::from_secs(10)).await;
			tracing::warn!("Chronicle balance is too low, retrying...");
		}

		Ok(spawner)
	}

	///
	/// Checks if wallet have enough balance
	async fn is_balance_available(&self) -> Result<bool> {
		let balance = self.wallet.balance().await?;
		Ok(balance > self.wallet_min_balance)
	}

	///
	/// look for gmp events and filter GmpCreated events from them
	async fn get_gmp_events_at(
		&self,
		gateway_contract: [u8; 20],
		from_block: Option<NonZeroU64>,
		to_block: u64,
	) -> Result<Vec<IGateway::GmpCreated>> {
		let logs = self
			.wallet
			.query(GetLogs {
				contracts: vec![gateway_contract.into()],
				topics: vec![IGateway::GmpCreated::SIGNATURE_HASH.0.into()],
				block: FilterBlockOption::Range {
					from_block: from_block.map(|x| x.get().into()),
					to_block: Some(to_block.into()),
				},
			})
			.await?
			.into_iter()
			.map(|log| {
				// if any GmpCreated message is received collect them and returns
				let topics =
					log.topics.into_iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
				let log = alloy_primitives::Log::new(
					gateway_contract.into(),
					topics,
					log.data.0.to_vec().into(),
				)
				.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
				let log = IGateway::GmpCreated::decode_log(&log, true)?;
				Ok::<IGateway::GmpCreated, anyhow::Error>(log.data)
			})
			.collect::<Result<Vec<_>, _>>()?;
		Ok(logs)
	}

	///
	/// executes the task function
	///
	/// # Arguments
	/// `function`: function of task
	/// `target_block_number`: block number of target chain (usable for read tasks)
	async fn execute_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<Payload> {
		Ok(match function {
			// execute the read function of task
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
			// reads the receipt for a transaction on target chain
			Function::EvmTxReceipt { tx } => {
				let Some(receipt) = self.wallet.eth_transaction_receipt(*tx).await? else {
					anyhow::bail!("transaction receipt from tx {} not found", hex::encode(tx));
				};
				if receipt.status_code != Some(1) {
					anyhow::bail!("transaction reverted");
				}
				let json = serde_json::json!({
					"transactionHash": format!("{:?}", receipt.transaction_hash),
					"transactionIndex": receipt.transaction_index,
					"blockHash": receipt.block_hash.0.map(|block_hash| format!("{block_hash:?}")),
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
			// executs the read message function. it looks for event emitted from gateway contracts
			Function::ReadMessages { batch_size } => {
				let network_id = self.network_id;
				let Some(gateway_contract) = self.substrate.get_gateway(network_id).await? else {
					anyhow::bail!("no gateway registered: skipped reading messages");
				};
				// gets gmp created events form contract and then convert it to a `Msg`
				let logs: Vec<_> = self
					.get_gmp_events_at(
						gateway_contract,
						NonZeroU64::new(target_block_number - batch_size.get() + 1),
						target_block_number,
					)
					.await
					.context("get_gmp_events_at")?
					.into_iter()
					.map(|event| Msg::from_event(event, network_id))
					.collect();
				tracing::info!("read target block: {:?}", target_block_number);
				if !logs.is_empty() {
					tracing::info!("read {} messages", logs.len());
				}
				Payload::Gmp(logs)
			},
			_ => anyhow::bail!("not a read function {function:?}"),
		})
	}

	async fn tss_sign(
		&self,
		block_number: BlockNumber,
		shard_id: ShardId,
		task_id: TaskId,
		task_phase: TaskPhase,
		payload: &[u8],
	) -> Result<(TssHash, TssSignature)> {
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssSigningRequest {
				request_id: TaskExecution::new(task_id, task_phase),
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
		tracing::debug!("debug_latency:{} executing read function", task_id);
		let result = self.execute_function(&function, target_block).await.map_err(|err| {
			tracing::error!("{:#?}", err);
			format!("{err}")
		});
		let payload = match result {
			Ok(payload) => payload,
			Err(payload) => Payload::Error(payload),
		};
		tracing::debug!("debug_latency:{} sending read payload for signing", task_id);
		let (_, signature) = self
			.tss_sign(block_num, shard_id, task_id, TaskPhase::Read, &payload.bytes(task_id))
			.await?;
		let result = TaskResult { shard_id, payload, signature };
		tracing::debug!("debug_latency:{} submitting task result", task_id);
		if let Err(e) = self.substrate.submit_task_result(task_id, result).await {
			tracing::error!("Error submitting task result {:?}", e);
		}
		Ok(())
	}

	async fn write(self, task_id: TaskId, function: Function) -> Result<()> {
		match self.is_balance_available().await {
			Ok(false) => {
				// unregister member
				if let Err(e) = self.substrate.submit_unregister_member().await {
					tracing::error!("Failed to unregister member: {:?}", e);
				};
				tracing::warn!("Chronicle balance too low, exiting");
				std::process::exit(1);
			},
			Ok(true) => {},
			Err(err) => tracing::error!("Could not fetch account balance: {:?}", err),
		}

		let submission = async move {
			match function {
				Function::EvmDeploy { bytecode } => {
					let _guard = self.wallet_guard.lock().await;
					self.wallet.eth_deploy_contract(bytecode.clone()).await
				},
				Function::EvmCall {
					address,
					input,
					amount,
					gas_limit,
				} => {
					let _guard = self.wallet_guard.lock().await;
					self.wallet.eth_send_call(address, input.clone(), amount, None, gas_limit).await
				},
				_ => anyhow::bail!("not a write function {function:?}"),
			}
		}
		.await
		.map(|s| s.tx_hash().0)
		.map_err(|err| err.to_string());
		if let Err(e) = self.substrate.submit_task_hash(task_id, submission).await {
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
		let (_, sig) = self
			.tss_sign(block_number, shard_id, task_id, TaskPhase::Sign, &payload)
			.await?;
		if let Err(e) = self.substrate.submit_task_signature(task_id, sig).await {
			tracing::error!("Error submitting task signature{:?}", e);
		}
		Ok(())
	}

	pub fn target_address(&self) -> &str {
		&self.wallet.account().address
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
							tracing::warn!("block stream closed {}", reason);
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
						tracing::debug!("error opening listener");
					},
					Poll::Ready(Err(err)) => {
						self.opening.take();
						tracing::debug!("error opening listener {}", err);
					},
					Poll::Pending => return Poll::Pending,
				}
			}
			let wallet = self.wallet;
			self.opening = Some(wallet.listen().boxed());
		}
	}
}
