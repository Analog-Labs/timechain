use crate::{metadata_scope, SubxtClient};
use anyhow::Result;
use time_primitives::{Payload, ShardId, TaskId, TaskPhase};

impl SubxtClient {
	pub async fn get_network_unassigned_tasks(&self, network_id: u16) -> Result<Vec<TaskId>> {
		let mut items = metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().unassigned_tasks_iter1(network_id);
			self.client.storage().at_latest().await?.iter(storage_query).await?
		});
		let mut tasks: Vec<TaskId> = vec![];
		while let Some(Ok(pair)) = items.next().await {
			tasks.push(pair.value);
		}
		Ok(tasks)
	}

	pub async fn is_task_complete(&self, task_id: u64) -> Result<bool> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().task_output(task_id);
			let Some(output) =
				self.client.storage().at_latest().await?.fetch(&storage_query).await?
			else {
				return Ok(false);
			};
			if let Payload::Error(msg) = output.payload.0 {
				anyhow::bail!("{msg}");
			}
			Ok(true)
		})
	}

	pub async fn get_task_phase(&self, task_id: u64) -> Result<Option<TaskPhase>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().task_phase_state(task_id);
			Ok(self
				.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage_query)
				.await?
				.map(|s| s.0))
		})
	}

	pub async fn get_task_shard(&self, task_id: u64) -> Result<Option<ShardId>> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().tasks().task_shard(task_id);
			Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
		})
	}
}
