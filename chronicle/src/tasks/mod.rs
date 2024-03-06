use anyhow::Result;
use async_trait::async_trait;
use futures::{Future, Stream};
use std::pin::Pin;
use time_primitives::{BlockHash, BlockNumber, Function, NetworkId, ShardId, TaskId, TssId};

pub mod executor;
pub mod spawner;

pub trait TaskSpawner: Clone + Send + Sync + 'static {
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;

	fn execute_read(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		function: Function,
		block_num: BlockNumber,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

	fn execute_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		payload: Vec<u8>,
		block_num: u32,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

	fn execute_write(
		&self,
		task_id: TaskId,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
}

#[async_trait]
pub trait TaskExecutor {
	fn network(&self) -> NetworkId;

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>>;

	async fn process_tasks(
		&mut self,
		block_hash: BlockHash,
		block_number: BlockNumber,
		shard_id: ShardId,
		target_block_height: u64,
	) -> Result<Vec<TssId>>;
}
