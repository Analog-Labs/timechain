use jsonrpsee::{
	core::{Error, RpcResult},
	proc_macros::rpc,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;
pub use time_primitives::{RpcTaskDetails, TaskCycle, TaskId, TasksRpcApi as TasksRuntimeApi};
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
	C::Api: TasksRuntimeApi<Block>,
{
	fn get_detail(
		&self,
		task_id: TaskId,
		cycle: Option<TaskCycle>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<RpcTaskDetails> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		match api.get_detail(at, task_id, cycle) {
			Ok(Some(result)) => Ok(result),
			Ok(None) => Err(Error::Custom("No task found".into())),
			Err(e) => Err(Error::Custom(format!("Error querying storage {}", e).into())),
		}
	}
}
