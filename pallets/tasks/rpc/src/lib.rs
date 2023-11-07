use jsonrpsee::{
	core::{Error, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
pub use time_primitives::{RpcTaskDetails, TaskCycle, TaskId, TasksApi};

#[rpc(client, server)]
pub trait TasksApi<BlockHash> {
	#[method(name = "tasks_getDetail")]
	fn get_detail(
		&self,
		task_id: TaskId,
		cycle: Option<TaskCycle>,
		at: Option<BlockHash>,
	) -> RpcResult<RpcTaskDetails>;
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
	#[error("Error querying runtime")]
	ErrorQueryingRuntime,
	#[error("TaskId '{0}' not found")]
	TaskNotFound(TaskId),
}

impl From<RpcError> for Error {
	fn from(value: RpcError) -> Self {
		match value {
			RpcError::ErrorQueryingRuntime => Error::to_call_error(RpcError::ErrorQueryingRuntime),
			RpcError::TaskNotFound(task_id) => {
				Error::to_call_error(RpcError::TaskNotFound(task_id))
			},
		}
	}
}
pub struct TasksRpcApi<C, Block> {
	_block: std::marker::PhantomData<Block>,
	client: Arc<C>,
}
impl<C, Block> TasksRpcApi<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			_block: Default::default(),
			client,
		}
	}
}

impl<C, Block> TasksApiServer<<Block as BlockT>::Hash> for TasksRpcApi<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: TasksApi<Block>,
{
	fn get_detail(
		&self,
		task_id: TaskId,
		cycle: Option<TaskCycle>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<RpcTaskDetails> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let description = api
			.get_task(at, task_id)
			.map_err(|_| RpcError::ErrorQueryingRuntime)?
			.ok_or(RpcError::TaskNotFound(task_id))?;
		let cycle_state =
			api.get_task_cycle(at, task_id).map_err(|_| RpcError::ErrorQueryingRuntime)?;
		let phase_state =
			api.get_task_phase(at, task_id).map_err(|_| RpcError::ErrorQueryingRuntime)?;
		let results = api
			.get_task_results(at, task_id, cycle)
			.map_err(|_| RpcError::ErrorQueryingRuntime)?;
		let results = results
			.iter()
			.map(|(cycle, result)| (*cycle, format!("0x{}", hex::encode(result.signature))))
			.collect::<Vec<_>>();
		let task_shard =
			api.get_task_shard(at, task_id).map_err(|_| RpcError::ErrorQueryingRuntime)?;
		Ok(RpcTaskDetails::new(description, cycle_state, phase_state, task_shard, results))
	}
}
