use crate::{timechain_runtime, SubxtClient};
use anyhow::Result;
use subxt::backend::StreamOfResults;
use timechain_runtime::runtime_types::time_primitives::task::{Payload, TaskDescriptor, TaskPhase};

impl SubxtClient {
	pub async fn get_tasks(&self) -> Result<StreamOfResults<(Vec<u8>, TaskDescriptor)>> {
		let storage_query = timechain_runtime::storage().tasks().tasks_iter();
		Ok(self.client.storage().at_latest().await?.iter(storage_query).await?)
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
