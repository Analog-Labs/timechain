use crate::{timechain_runtime, SubxtClient};

use time_primitives::{TaskCycle, TaskError, TaskId, TaskResult, TasksPayload, TssSignature};
use timechain_runtime::runtime_types::time_primitives::task;

impl TasksPayload for SubxtClient {
	fn submit_task_error(&self, task_id: TaskId, cycle: TaskCycle, error: TaskError) -> Vec<u8> {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
		self.make_transaction(&tx)
	}

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Vec<u8> {
		let tx = timechain_runtime::tx().tasks().submit_signature(task_id, signature);
		self.make_transaction(&tx)
	}

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> Vec<u8> {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
		self.make_transaction(&tx)
	}

	fn submit_task_result(&self, task_id: TaskId, cycle: TaskCycle, status: TaskResult) -> Vec<u8> {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
		self.make_transaction(&tx)
	}
}
