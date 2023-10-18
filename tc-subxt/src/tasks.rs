use crate::{timechain_runtime, SubxtClient, TcSubxtError};

use time_primitives::{TaskCycle, TaskError, TaskId, TaskResult};
use timechain_runtime::runtime_types::time_primitives::task;

impl SubxtClient {
	pub fn submit_task_error(
		&mut self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> Result<Vec<u8>, TcSubxtError> {
		let error: task::TaskError = unsafe { std::mem::transmute(error) };
		let tx = timechain_runtime::tx().tasks().submit_error(task_id, cycle, error);
		self.make_transaction(&tx)
	}

	pub fn submit_task_hash(
		&mut self,
		task_id: TaskId,
		cycle: TaskCycle,
		hash: Vec<u8>,
	) -> Result<Vec<u8>, TcSubxtError> {
		let tx = timechain_runtime::tx().tasks().submit_hash(task_id, cycle, hash);
		self.make_transaction(&tx)
	}

	pub fn submit_task_result(
		&mut self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> Result<Vec<u8>, TcSubxtError> {
		let status: task::TaskResult = unsafe { std::mem::transmute(status) };
		let tx = timechain_runtime::tx().tasks().submit_result(task_id, cycle, status);
		self.make_transaction(&tx)
	}
}
