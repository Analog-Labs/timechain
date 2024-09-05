use anyhow::{Context as _, Result};
use connector::Connector;
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
use schnorr_evm::VerifyingKey;
use std::time::Duration;
use std::{future::Future, num::NonZeroU64, path::PathBuf, pin::Pin};
use time_primitives::{
	BlockNumber, Function, GatewayMessage, GmpParams, IConnector, NetworkId, Payload, Runtime,
	ShardId, TaskExecution, TaskId, TaskPhase, TaskResult, TssHash, TssSignature,
	TssSigningRequest,
};
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
	connector: Connector,
	substrate: S,
	min_balance: u128,
}

impl<S> TaskSpawner<S>
where
	S: Runtime,
{
	pub async fn new(params: TaskSpawnerParams<S>) -> Result<Self> {
		let connector = Connector::new(
			params.network_id,
			&params.blockchain,
			&params.network,
			&params.url,
			&params.keyfile,
		)
		.await?;
		while connector.balance().await? < params.min_balance {
			sleep(Duration::from_secs(10)).await;
			tracing::warn!("Chronicle balance is too low, retrying...");
		}
		Ok(Self {
			tss: params.tss,
			substrate: params.substrate,
			connector,
			min_balance: params.min_balance,
		})
	}

	pub fn target_address(&self) -> &str {
		self.connector.account()
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

	/// executes the task function
	///
	/// # Arguments
	/// `function`: function of task
	/// `target_block_number`: block number of target chain (usable for read tasks)
	async fn execute_read_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<Payload> {
		Ok(match function {
			// reads the receipt for a transaction on target chain
			Function::GatewayMessageReceipt { tx } => {
				let result = self.connector.verify_submission(*tx).await?;
				Payload::Hashed(VerifyingKey::message_hash(&result))
			},
			// executs the read message function. it looks for event emitted from gateway contracts
			Function::ReadMessages { batch_size } => {
				let network_id = self.connector.network_id();
				let Some(gateway_contract) = self.substrate.get_gateway(network_id).await? else {
					anyhow::bail!("no gateway registered: skipped reading messages");
				};
				// gets gmp created events form contract and then convert it to a `Msg`
				let logs: Vec<_> = self
					.connector
					.read_messages(
						gateway_contract,
						NonZeroU64::new(target_block_number - batch_size.get() + 1),
						target_block_number,
					)
					.await
					.context("get_gmp_events_at")?;
				tracing::info!("read target block: {:?}", target_block_number);
				if !logs.is_empty() {
					tracing::info!("read {} messages", logs.len());
				}
				Payload::Gmp(logs)
			},
			_ => anyhow::bail!("not a read function {function:?}"),
		})
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
		tracing::debug!(task_id = task_id, shard_id = shard_id, "executing read function",);
		let result = self.execute_read_function(&function, target_block).await.map_err(|err| {
			tracing::error!(task_id = task_id, shard_id = shard_id, "{:#?}", err);
			format!("{err}")
		});
		let payload = match result {
			Ok(payload) => payload,
			Err(payload) => Payload::Error(payload),
		};
		tracing::debug!(task_id = task_id, shard_id = shard_id, "sending read payload for signing",);
		let (_, signature) = self
			.tss_sign(block_num, shard_id, task_id, TaskPhase::Read, &payload.bytes(task_id))
			.await?;
		let result = TaskResult { shard_id, payload, signature };
		tracing::debug!(task_id = task_id, shard_id = shard_id, "submitting task result",);
		if let Err(e) = self.substrate.submit_task_result(task_id, result).await {
			tracing::error!(
				task_id = task_id,
				shard_id = shard_id,
				"Error submitting task result {:?}",
				e
			);
		}
		Ok(())
	}

	async fn write(self, params: GmpParams, msg: GatewayMessage, sig: TssSignature) -> Result<()> {
		match self.connector.balance().await {
			Ok(balance) if balance < self.min_balance => {
				// unregister member
				if let Err(e) = self.substrate.submit_unregister_member().await {
					tracing::error!(task_id = msg.task_id, "Failed to unregister member: {:?}", e);
				};
				tracing::warn!(task_id = msg.task_id, "Chronicle balance too low, exiting");
				std::process::exit(1);
			},
			Ok(_) => {},
			Err(err) => {
				tracing::error!(task_id = msg.task_id, "Could not fetch account balance: {:?}", err)
			},
		}
		let task_id = msg.task_id;
		let hash = self.connector.submit_messages(params.gateway, msg, params.signer, sig).await;
		if let Err(e) = self.substrate.submit_task_hash(task_id, hash).await {
			tracing::error!(task_id = task_id, "Error submitting task hash {:?}", e);
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
			tracing::error!(
				task_id = task_id,
				shard_id = shard_id,
				"Error submitting task signature {:?}",
				e
			);
		}
		Ok(())
	}
}

impl<S> super::TaskSpawner for TaskSpawner<S>
where
	S: Runtime,
{
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		self.connector.block_stream()
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
		params: GmpParams,
		msg: GatewayMessage,
		sig: TssSignature,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().write(params, msg, sig).boxed()
	}
}
