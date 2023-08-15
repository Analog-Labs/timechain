use crate::worker::TaskExecutor;
use sc_client_api::{Backend, BlockchainEvents};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::{PeerId, TaskSpawner, TimeApi};

mod worker;

pub use crate::worker::{Task, TaskSpawnerParams};

#[cfg(test)]
mod tests;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "task-executor";

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<B: Block, BE, C, R, T>
where
	B: Block,
	BE: Backend<B> + 'static,
	C: BlockchainEvents<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
	T: TaskSpawner,
{
	pub _block: PhantomData<B>,
	pub backend: Arc<BE>,
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub peer_id: PeerId,
	pub task_spawner: T,
}

/// Start the task Executor gadget.
///
/// This is a thin shim around running and awaiting a task Executor.
pub async fn start_task_executor_gadget<B, BE, C, R, T>(params: TaskExecutorParams<B, BE, C, R, T>)
where
	B: Block,
	BE: Backend<B> + 'static,
	C: BlockchainEvents<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
	T: TaskSpawner,
{
	let mut worker = TaskExecutor::new(params);
	worker.run().await;
}
