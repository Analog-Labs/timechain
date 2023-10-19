use anyhow::Result;
use futures::{Future, Stream};
use std::pin::Pin;
use time_primitives::{
	BlockHash, BlockNumber, Function, Network, ShardId, TaskCycle, TaskId, TssId,
};

pub mod executor;
pub mod spawner;

#[cfg(test)]
mod tests;

pub trait TaskSpawner {
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;

	#[allow(clippy::too_many_arguments)]
	fn execute_read(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		function: Function,
		hash: String,
		block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

	fn execute_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		payload: Vec<u8>,
		block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

	fn execute_write(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
}

pub trait TaskExecutor {
	fn network(&self) -> Network;

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;

	fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<Vec<TssId>>;
}
