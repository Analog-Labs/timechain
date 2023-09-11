use crate::{AccountId, Network, ShardId, TssSignature};
use anyhow::Result;
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::Serialize;
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use std::future::Future;
#[cfg(feature = "std")]
use std::pin::Pin;

pub type TaskId = u64;
pub type TaskCycle = u64;
pub type TaskRetryCount = u8;
pub type LastExecutedBlockNum = u64;

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EVMViewWithoutAbi { address: String, function_signature: String, input: Vec<String> },
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct CycleStatus {
	pub shard_id: ShardId,
	pub signature: TssSignature,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskError {
	pub shard_id: ShardId,
	pub error: String,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskDescriptor {
	pub owner: AccountId,
	pub network: Network,
	pub function: Function,
	pub cycle: TaskCycle,
	pub start: u64,
	pub period: u64,
	pub hash: String,
}

impl TaskDescriptor {
	pub fn trigger(&self, cycle: TaskCycle) -> u64 {
		self.start + cycle * self.period
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskDescriptorParams {
	pub network: Network,
	pub cycle: TaskCycle,
	pub start: u64,
	pub period: u64,
	pub hash: String,
	pub function: Function,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum TaskStatus {
	Created,
	Failed { error: TaskError },
	Stopped,
	Completed,
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq, Eq, Hash)]
pub struct TaskExecution {
	pub task_id: TaskId,
	pub cycle: TaskCycle,
	pub retry_count: TaskRetryCount,
	pub last_block_num: Option<LastExecutedBlockNum>
}

impl TaskExecution {
	pub fn new(task_id: TaskId, cycle: TaskCycle, retry_count: TaskRetryCount, last_block_num:  Option<LastExecutedBlockNum>) -> Self {
		Self { task_id, cycle, retry_count, last_block_num }
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for TaskExecution {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}/{}/{}/{:?}", self.task_id, self.cycle, self.retry_count, self.last_block_num)
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait TaskSpawner {
	async fn block_height(&self) -> Result<u64>;

	fn execute(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		retry_count: TaskRetryCount,
		task: TaskDescriptor,
		block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>>;
}
