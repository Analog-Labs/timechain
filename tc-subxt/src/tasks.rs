use crate::{timechain_runtime, SubxtClient};
use anyhow::Result;
use time_primitives::TaskId;
use timechain_runtime::runtime_types::time_primitives::task::{Payload, TaskPhase};

impl SubxtClient {
	pub async fn get_network_unassigned_tasks(&self, network_id: u16) -> Result<Vec<TaskId>> {
		let storage_query = timechain_runtime::storage().tasks().unassigned_tasks_iter1(network_id);
		let mut items = self.client.storage().at_latest().await?.iter(storage_query).await?;
		let mut tasks: Vec<TaskId> = vec![];
		while let Some(Ok((key, _))) = items.next().await {
			// getting the task_id from last 8 bytes
			let s = &key[key.len() - 8..];
			let v: [u8; 8] = s.try_into()?;
			let val = u64::from_le_bytes(v);
			tasks.push(val);
		}
		Ok(tasks)
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
