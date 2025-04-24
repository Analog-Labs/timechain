use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use subxt::utils::H256;
use time_primitives::{
	BatchId, BlockHash, ErrorMsg, GatewayMessage, GmpEvents, Hash, MessageId, NetworkId, ShardId,
	Task, TaskId, TaskResult, TssPublicKey,
};

impl SubxtClient {
	pub async fn task(&self, task_id: TaskId, block: BlockHash) -> Result<Option<Task>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().tasks_api().get_task(task_id);
		let task = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(task.map(|s| s.0))
	}

	pub async fn task_network(
		&self,
		task_id: TaskId,
		block: BlockHash,
	) -> Result<Option<NetworkId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().task_network(task_id);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn assigned_tasks(&self, shard: ShardId, block: BlockHash) -> Result<Vec<TaskId>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().tasks_api().get_shard_tasks(shard);
		Ok(self.client.runtime_api().at(block).call(runtime_call).await?)
	}

	pub async fn get_failed_batches(&self, block: BlockHash) -> Result<Vec<BatchId>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().tasks_api().get_failed_batches();
		Ok(self.client.runtime_api().at(block).call(runtime_call).await?)
	}

	pub async fn get_pending_batches(&self, block: BlockHash) -> Result<Vec<BatchId>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().tasks_api().get_pending_batches();
		Ok(self.client.runtime_api().at(block).call(runtime_call).await?)
	}

	pub async fn unassigned_tasks(
		&self,
		network: NetworkId,
		block: BlockHash,
	) -> Result<Vec<TaskId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().ua_tasks_iter1(network);
		let mut items = self.client.storage().at(block).iter(storage_query).await?;
		let mut tasks: Vec<TaskId> = vec![];
		while let Some(Ok(pair)) = items.next().await {
			tasks.push(pair.value);
		}
		Ok(tasks)
	}

	pub async fn assigned_shard(
		&self,
		task_id: TaskId,
		block: BlockHash,
	) -> Result<Option<ShardId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().task_shard(task_id);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn task_output(
		&self,
		task_id: TaskId,
		block: BlockHash,
	) -> Result<Option<Result<(), ErrorMsg>>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().task_output(task_id);
		let output = self.client.storage().at(block).fetch(&storage_query).await?;
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
		block: BlockHash,
	) -> Result<Option<TaskId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().read_events_task(network);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn batch_message(
		&self,
		batch: BatchId,
		block: BlockHash,
	) -> Result<Option<GatewayMessage>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().tasks_api().get_batch_message(batch);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(data.map(|s| s.0))
	}

	pub async fn batch_task(&self, batch: BatchId, block: BlockHash) -> Result<Option<TaskId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().batch_task_id(batch);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn batch_tx_hash(&self, batch: BatchId, block: BlockHash) -> Result<Option<Hash>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().batch_tx_hash(batch);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn message_received_task(
		&self,
		message: MessageId,
		block: BlockHash,
	) -> Result<Option<TaskId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().message_received_task_id(message);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn message_batch(
		&self,
		message: MessageId,
		block: BlockHash,
	) -> Result<Option<BatchId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().message_batch_id(message);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn message_executed_task(
		&self,
		message: MessageId,
		block: BlockHash,
	) -> Result<Option<TaskId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().message_executed_task_id(message);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn shard_register_batch(
		&self,
		key: TssPublicKey,
		block: BlockHash,
	) -> Result<Option<BatchId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().shard_register_batch_id(key);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn shard_unregister_batch(
		&self,
		key: TssPublicKey,
		block: BlockHash,
	) -> Result<Option<BatchId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().shard_unregister_batch_id(key);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn is_shard_registered(&self, key: TssPublicKey, block: BlockHash) -> Result<bool> {
		let block = H256(block.0);
		let storage_query = metadata::storage().tasks().shard_registered(key);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?.is_some())
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
