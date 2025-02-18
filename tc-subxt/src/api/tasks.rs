use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{
	BatchId, ErrorMsg, GatewayMessage, GmpEvents, Hash, MessageId, NetworkId, PublicKey, ShardId,
	Task, TaskId, TaskResult,
};

impl SubxtClient {
	pub async fn task(&self, task_id: TaskId) -> Result<Option<Task>> {
		let runtime_call = metadata::apis().tasks_api().get_task(task_id);
		let task = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(task.map(|s| s.0))
	}

	pub async fn task_network(&self, task_id: TaskId) -> Result<Option<NetworkId>> {
		let storage_query = metadata::storage().tasks().task_network(task_id);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn task_submitter(&self, task_id: TaskId) -> Result<Option<PublicKey>> {
		let runtime_call = metadata::apis().tasks_api().get_task_submitter(task_id);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data.map(|s| s.0))
	}

	pub async fn assigned_tasks(&self, shard: ShardId) -> Result<Vec<TaskId>> {
		let runtime_call = metadata::apis().tasks_api().get_shard_tasks(shard);
		Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
	}

	pub async fn get_failed_tasks(&self) -> Result<Vec<TaskId>> {
		let runtime_call = metadata::apis().tasks_api().get_failed_tasks();
		Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
	}

	pub async fn unassigned_tasks(&self, network: NetworkId) -> Result<Vec<TaskId>> {
		let storage_query = metadata::storage().tasks().ua_tasks_iter1(network);
		let mut items = self.client.storage().at_latest().await?.iter(storage_query).await?;
		let mut tasks: Vec<TaskId> = vec![];
		while let Some(Ok(pair)) = items.next().await {
			tasks.push(pair.value);
		}
		Ok(tasks)
	}

	pub async fn assigned_shard(&self, task_id: TaskId) -> Result<Option<ShardId>> {
		let storage_query = metadata::storage().tasks().task_shard(task_id);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn task_output(&self, task_id: TaskId) -> Result<Option<Result<(), ErrorMsg>>> {
		let storage_query = metadata::storage().tasks().task_output(task_id);
		let output = self.client.storage().at_latest().await?.fetch(&storage_query).await?;
		let output_converted = match output {
			Some(Ok(())) => Some(Ok(())),
			Some(Err(static_err)) => Some(Err((*static_err).clone())),
			None => None,
		};
		Ok(output_converted)
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

	pub async fn read_events_task(&self, network: NetworkId) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().read_events_task(network);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn batch_message(&self, batch: BatchId) -> Result<Option<GatewayMessage>> {
		let runtime_call = metadata::apis().tasks_api().get_batch_message(batch);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data.map(|s| s.0))
	}

	pub async fn batch_task(&self, batch: BatchId) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().batch_task_id(batch);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn batch_tx_hash(&self, batch: BatchId) -> Result<Option<Hash>> {
		let storage_query = metadata::storage().tasks().batch_tx_hash(batch);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn message_received_task(&self, message: MessageId) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().message_received_task_id(message);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn message_batch(&self, message: MessageId) -> Result<Option<BatchId>> {
		let storage_query = metadata::storage().tasks().message_batch_id(message);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn message_executed_task(&self, message: MessageId) -> Result<Option<TaskId>> {
		let storage_query = metadata::storage().tasks().message_executed_task_id(message);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}
}
