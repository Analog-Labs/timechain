use crate::worker::Tx;
use crate::{metadata_scope, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{
	BatchId, GatewayMessage, MessageId, NetworkId, PublicKey, ShardId, Task, TaskId, TaskResult,
};

impl SubxtClient {
	pub async fn task(&self, task_id: TaskId) -> Result<Option<Task>> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().tasks_api().get_task(task_id);
			let task = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
			Ok(task.map(|s| s.0))
		})
	}

	pub async fn task_network(&self, task_id: TaskId) -> Result<Option<NetworkId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().task_network(task_id);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}

	pub async fn task_submitter(&self, task_id: TaskId) -> Result<Option<PublicKey>> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().tasks_api().get_task_submitter(task_id);
			let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
			Ok(data.map(|s| s.0))
		})
	}

	pub async fn assigned_tasks(&self, shard: ShardId) -> Result<Vec<TaskId>> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().tasks_api().get_shard_tasks(shard);
			Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
		})
	}

	pub async fn unassigned_tasks(&self, network: NetworkId) -> Result<Vec<TaskId>> {
		let mut items = metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().ua_tasks_iter1(network);
			self.client.storage().at_latest().await?.iter(storage_query).await?
		});
		let mut tasks: Vec<TaskId> = vec![];
		while let Some(Ok(pair)) = items.next().await {
			tasks.push(pair.value);
		}
		Ok(tasks)
	}

	pub async fn assigned_shard(&self, task_id: TaskId) -> Result<Option<ShardId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().task_shard(task_id);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}

	pub async fn task_output(&self, task_id: TaskId) -> Result<Option<Result<(), String>>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().task_output(task_id);
			let output = self.client.storage().at_latest().await?.fetch(&storage_query).await?;
			Ok(output)
		})
	}

	pub async fn submit_task_result(&self, task_id: TaskId, result: TaskResult) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::SubmitTaskResult { task_id, result }, tx))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}

	pub async fn read_events_task(&self, network: NetworkId) -> Result<Option<TaskId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().read_events_task(network);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}

	pub async fn batch_message(&self, batch: BatchId) -> Result<Option<GatewayMessage>> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().tasks_api().get_batch_message(batch);
			let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
			Ok(data.map(|s| s.0))
		})
	}

	pub async fn batch_task(&self, batch: BatchId) -> Result<Option<TaskId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().batch_task_id(batch);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}

	pub async fn message_received_task(&self, message: MessageId) -> Result<Option<TaskId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().message_received_task_id(message);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}

	pub async fn message_batch(&self, message: MessageId) -> Result<Option<BatchId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().message_batch_id(message);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}

	pub async fn message_executed_task(&self, message: MessageId) -> Result<Option<TaskId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().message_executed_task_id(message);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}
}
