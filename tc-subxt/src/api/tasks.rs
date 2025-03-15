use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{
	BatchId, BlockHash, ErrorMsg, GatewayMessage, GmpEvents, Hash, MessageId, NetworkId, PublicKey,
	ShardId, Task, TaskId, TaskResult, TssPublicKey,
};

impl SubxtClient {
	pub async fn task(&self, task_id: TaskId, block: Option<BlockHash>) -> Result<Option<Task>> {
		let runtime_call = metadata::apis().tasks_api().get_task(task_id);
		let task = self.rt_at_or_latest(block).await?.call(runtime_call).await?;
		Ok(task.map(|s| s.0))
	}

	pub async fn task_network(
		&self,
		task_id: TaskId,
		block: Option<BlockHash>,
	) -> Result<Option<NetworkId>> {
		let storage_query = metadata::storage().tasks().task_network(task_id);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn task_submitter(
		&self,
		task_id: TaskId,
		block: Option<BlockHash>,
	) -> Result<Option<PublicKey>> {
		let runtime_call = metadata::apis().tasks_api().get_task_submitter(task_id);
		let data = self.rt_at_or_latest(block).await?.call(runtime_call).await?;
		Ok(data.map(|s| s.0))
	}

	pub async fn assigned_tasks(
		&self,
		shard: ShardId,
		block: Option<BlockHash>,
	) -> Result<Vec<TaskId>> {
		let runtime_call = metadata::apis().tasks_api().get_shard_tasks(shard);
		Ok(self.rt_at_or_latest(block).await?.call(runtime_call).await?)
	}

	pub async fn get_failed_tasks(&self, block: Option<BlockHash>) -> Result<Vec<TaskId>> {
		let runtime_call = metadata::apis().tasks_api().get_failed_tasks();
		Ok(self.rt_at_or_latest(block).await?.call(runtime_call).await?)
	}

	pub async fn unassigned_tasks(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<Vec<TaskId>> {
		let storage_query = metadata::storage().tasks().ua_tasks_iter1(network);
		let mut items = self.st_at_or_latest(block).await?.iter(storage_query).await?;
		let mut tasks: Vec<TaskId> = vec![];
		while let Some(Ok(pair)) = items.next().await {
			tasks.push(pair.value);
		}
		Ok(tasks)
	}

	pub async fn assigned_shard(
		&self,
		task_id: TaskId,
		block: Option<BlockHash>,
	) -> Result<Option<ShardId>> {
		let storage_query = metadata::storage().tasks().task_shard(task_id);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn task_output(
		&self,
		task_id: TaskId,
		block: Option<BlockHash>,
	) -> Result<Option<Result<(), ErrorMsg>>> {
		let storage_query = metadata::storage().tasks().task_output(task_id);
		let output = self.st_at_or_latest(block).await?.fetch(&storage_query).await?;
		let output_converted = match output {
			Some(Ok(())) => Some(Ok(())),
			Some(Err(static_err)) => Some(Err((*static_err).clone())),
			None => None,
		};
		Ok(output_converted)
	}

	pub async fn read_events_task(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().read_events_task(network);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn batch_message(
		&self,
		batch: BatchId,
		block: Option<BlockHash>,
	) -> Result<Option<GatewayMessage>> {
		let runtime_call = metadata::apis().tasks_api().get_batch_message(batch);
		let data = self.rt_at_or_latest(block).await?.call(runtime_call).await?;
		Ok(data.map(|s| s.0))
	}

	pub async fn batch_task(
		&self,
		batch: BatchId,
		block: Option<BlockHash>,
	) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().batch_task_id(batch);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn batch_tx_hash(
		&self,
		batch: BatchId,
		block: Option<BlockHash>,
	) -> Result<Option<Hash>> {
		let storage_query = metadata::storage().tasks().batch_tx_hash(batch);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn message_received_task(
		&self,
		message: MessageId,
		block: Option<BlockHash>,
	) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().message_received_task_id(message);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn message_batch(
		&self,
		message: MessageId,
		block: Option<BlockHash>,
	) -> Result<Option<BatchId>> {
		let storage_query = metadata::storage().tasks().message_batch_id(message);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn message_executed_task(
		&self,
		message: MessageId,
		block: Option<BlockHash>,
	) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().message_executed_task_id(message);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn shard_register_batch(
		&self,
		key: TssPublicKey,
		block: Option<BlockHash>,
	) -> Result<Option<BatchId>> {
		let storage_query = metadata::storage().tasks().shard_register_batch_id(key);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn shard_unregister_batch(
		&self,
		key: TssPublicKey,
		block: Option<BlockHash>,
	) -> Result<Option<BatchId>> {
		let storage_query = metadata::storage().tasks().shard_unregister_batch_id(key);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::SubmitTaskResult { task_id, result }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn submit_gmp_events(&self, network: NetworkId, gmp_events: GmpEvents) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::SubmitGmpEvents { network, gmp_events }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn remove_task(&self, task_id: TaskId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::RemoveTask { task_id }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn restart_failed_batch(&self, batch_id: TaskId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::RestartBatch { batch_id }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}
}
