use anyhow::Result;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::{Network, PublicKey, ShardId, TaskSpawner, TasksApi};
use tokio::sync::Mutex;

mod worker;

pub use crate::worker::{Task, TaskSpawnerParams};

#[cfg(test)]
mod tests;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "task-executor";

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<B: Block, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	R::Api: TasksApi<B>,
	T: TaskSpawner,
{
	pub _block: PhantomData<B>,
	pub runtime: Arc<R>,
	pub task_spawner: T,
	pub network: Network,
	pub public_key: PublicKey,
}

pub struct TaskExecutor<B: Block, R, T> {
	network: Network,
	task_executor: Arc<Mutex<worker::TaskExecutor<B, R, T>>>,
}

impl<B, R, T> TaskExecutor<B, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	T: TaskSpawner + Send + Sync + 'static,
{
	pub fn new(params: TaskExecutorParams<B, R, T>) -> Self {
		Self {
			network: params.network,
			task_executor: Arc::new(Mutex::new(worker::TaskExecutor::new(params))),
		}
	}
}

impl<B: Block, R, T> Clone for TaskExecutor<B, R, T> {
	fn clone(&self) -> Self {
		Self {
			network: self.network,
			task_executor: self.task_executor.clone(),
		}
	}
}

#[async_trait::async_trait]
impl<B, R, T> time_primitives::TaskExecutor<B> for TaskExecutor<B, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: TasksApi<B>,
	T: TaskSpawner + Send + Sync + 'static,
{
	fn network(&self) -> Network {
		self.network
	}

	async fn start_tasks(
		&self,
		block_hash: B::Hash,
		block_num: u64,
		shard_id: ShardId,
	) -> Result<()> {
		self.task_executor
			.lock()
			.await
			.start_tasks(block_hash, block_num, shard_id)
			.await
	}
}
