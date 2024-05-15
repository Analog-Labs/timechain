use crate::{timechain_runtime, SubxtClient};
use anyhow::Result;
use time_primitives::TaskId;
use timechain_runtime::runtime_types::time_primitives::task::{Payload, TaskPhase};

impl SubxtClient {
	pub async fn get_network_unassigned_tasks(&self, network_id: u16) -> Result<Vec<TaskId>> {
		let storage_query = timechain_runtime::storage().tasks().unassigned_tasks(network_id);
		let items = self.client.storage().at_latest().await?.fetch(&storage_query).await?;
		Ok(items.unwrap_or_default())
	}

	pub async fn is_task_complete(&self, task_id: u64) -> Result<bool> {
		let storage_query = timechain_runtime::storage().tasks().task_output(task_id);
		let Some(output) = self.client.storage().at_latest().await?.fetch(&storage_query).await?
		else {
			return Ok(false);
		};
		if let Payload::Error(msg) = output.payload {
			anyhow::bail!("{msg}");
		}
		Ok(true)
	}

	pub async fn get_task_phase(&self, task_id: u64) -> Result<Option<TaskPhase>> {
		let storage_query = timechain_runtime::storage().tasks().task_phase_state(task_id);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}
}
