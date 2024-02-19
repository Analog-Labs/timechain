use crate::timechain_runtime::runtime_types::timechain_runtime::RuntimeCall;
use crate::{timechain_runtime, SubxtClient};
use anyhow::Result;
use subxt::backend::StreamOfResults;
use subxt::tx::Payload;
use timechain_runtime::runtime_types::time_primitives::task::{
	TaskDescriptor, TaskDescriptorParams, TaskStatus,
};
use timechain_runtime::tasks::calls::types::CreateTask;

impl SubxtClient {
	pub fn create_task_payload(task: TaskDescriptorParams) -> Payload<CreateTask> {
		timechain_runtime::tx().tasks().create_task(task)
	}

	//sudo call
	pub fn create_register_gateway(shard_id: u64, address: [u8; 20]) -> RuntimeCall {
		RuntimeCall::Tasks(
			timechain_runtime::runtime_types::pallet_tasks::pallet::Call::register_gateway {
				bootstrap: shard_id,
				address,
			},
		)
	}

	pub async fn get_tasks(&self) -> Result<StreamOfResults<(Vec<u8>, TaskDescriptor)>> {
		let storage_query = timechain_runtime::storage().tasks().tasks_iter();
		Ok(self.client.storage().at_latest().await?.iter(storage_query).await?)
	}

	pub async fn get_task_state(&self, task_id: u64) -> Result<Option<TaskStatus>> {
		let storage_query = timechain_runtime::storage().tasks().task_state(task_id);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}
}
