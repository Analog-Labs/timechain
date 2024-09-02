use anyhow::{Context as _, Result};
use connector::Connector;
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, Stream};
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
	network_id: NetworkId,
}

impl<S> TaskSpawner<S>
where
	S: Runtime,
{
	pub async fn new(params: TaskSpawnerParams<S>) -> Result<Self> {
		let connector = Connector::new(
			&params.blockchain,
			&params.network,
			&params.url,
			&params.keyfile,
			params.min_balance,
		)
		.await?;
		Ok(Self {
			tss: params.tss,
			substrate: params.substrate,
			network_id: params.network_id,
			connector,
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
		tracing::debug!(task_id = task_id, shard_id = shard_id, "executing read function",);
		let result = self.execute_function(&function, target_block).await.map_err(|err| {
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

	async fn write(self, task_id: TaskId, function: Function) -> Result<()> {
		match self.is_balance_available().await {
			Ok(false) => {
				// unregister member
				if let Err(e) = self.substrate.submit_unregister_member().await {
					tracing::error!(task_id = task_id, "Failed to unregister member: {:?}", e);
				};
				tracing::warn!(task_id = task_id, "Chronicle balance too low, exiting");
				std::process::exit(1);
			},
			Ok(true) => {},
			Err(err) => {
				tracing::error!(task_id = task_id, "Could not fetch account balance: {:?}", err)
			},
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
		task_id: TaskId,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.clone().write(task_id, function).boxed()
	}
}
