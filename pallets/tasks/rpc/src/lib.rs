use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
pub use time_primitives::{TaskCycle, TaskId, TaskRpcDetails, TasksRpcApi as TasksRuntimeApi};
#[rpc(client, server)]
pub trait TasksApi<BlockHash> {
	#[method(name = "tasks_getDetail")]
	fn get_detail(
		&self,
		task_id: TaskId,
		cycle: Option<TaskCycle>,
		at: Option<BlockHash>,
	) -> RpcResult<Option<TaskRpcDetails>>;
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
	) -> RpcResult<Option<TaskRpcDetails>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let details = api.get_detail(at, task_id, cycle).unwrap();
		Ok(details)
	}
}
