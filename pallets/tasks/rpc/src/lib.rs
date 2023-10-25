use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
pub use time_primitives::{TaskId, TasksRpcApi as TasksRuntimeApi};
#[rpc(client, server)]
pub trait TasksApi<BlockHash> {
	#[method(name = "tasks_getDetail")]
	fn get_detail(&self, at: Option<BlockHash>, task_id: TaskId) -> RpcResult<u64>;
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
	fn get_detail(&self, at: Option<<Block as BlockT>::Hash>, task_id: TaskId) -> RpcResult<u64> {
		let api = self.client.runtime_api();
		// let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
		Ok(0)
	}
}
