use crate::{timechain_runtime, SubxtClient};
use anyhow::Result;
use subxt::backend::StreamOfResults;
use subxt::tx::Payload;
use time_primitives::{TaskCycle, TaskError, TaskId, TaskResult, TasksPayload, TssSignature};
use timechain_runtime::runtime_types::time_primitives::task;
use timechain_runtime::runtime_types::time_primitives::task::{
	TaskDescriptor, TaskDescriptorParams, TaskStatus,
};
use timechain_runtime::tasks::calls::types::CreateTask;

impl SubxtClient {
	pub fn create_task_payload(task: TaskDescriptorParams) -> Payload<CreateTask> {
		timechain_runtime::tx().tasks().create_task(task)
	}

	pub async fn get_tasks(&self) -> Result<StreamOfResults<(Vec<u8>, TaskDescriptor)>> {
		let storage_query = timechain_runtime::storage().tasks().tasks_iter();
		Ok(self.client.storage().at_latest().await?.iter(storage_query).await?)
	}

	pub async fn get_task_state(&self, task_id: u64) -> Result<Option<TaskStatus>> {
		let storage_query = timechain_runtime::storage().tasks().task_state(task_id);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn get_task_cycle(&self, task_id: u64) -> Result<Option<u64>> {
		let storage_query = timechain_runtime::storage().tasks().task_cycle_state(task_id);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}
}

impl TasksPayload for SubxtClient {
	fn submit_task_error(&self, task_id: TaskId, cycle: TaskCycle, error: TaskError) -> Vec<u8> {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
		self.create_signed_payload(&tx)
	}

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Vec<u8> {
		let tx = timechain_runtime::tx().tasks().submit_signature(task_id, signature);
		self.create_signed_payload(&tx)
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> Vec<u8> {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
		self.create_signed_payload(&tx)
	}

	fn submit_task_result(&self, task_id: TaskId, cycle: TaskCycle, status: TaskResult) -> Vec<u8> {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
		self.create_signed_payload(&tx)
	}
}
