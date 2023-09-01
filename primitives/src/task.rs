use crate::{AccountId, Network, PublicKey, ShardId, TssSignature};
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

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EvmDeploy { bytecode: Vec<u8> },
	EvmCall { address: String, function_signature: String, input: Vec<String>, amount: u128 },
	EvmViewCall { address: String, function_signature: String, input: Vec<String> },
	EvmTxReceipt { tx: String },
}

impl Function {
	pub fn is_payable(&self) -> bool {
		matches!(self, Self::EvmDeploy { .. } | Self::EvmCall { .. })
	}
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
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPhase {
	Write(PublicKey),
	Read(Option<String>),
}

impl TaskPhase {
	pub fn public_key(&self) -> Option<&PublicKey> {
		if let Self::Write(public_key) = self {
			Some(public_key)
		} else {
			None
		}
	}

	pub fn tx_hash(&self) -> Option<&str> {
		if let Self::Read(Some(tx_hash)) = self {
			Some(tx_hash.as_str())
		} else {
			None
		}
	}
}

impl Default for TaskPhase {
	fn default() -> Self {
		TaskPhase::Read(None)
	}
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskExecution {
	pub task_id: TaskId,
	pub cycle: TaskCycle,
	pub retry_count: TaskRetryCount,
	pub phase: TaskPhase,
}

impl TaskExecution {
	pub fn new(
		task_id: TaskId,
		cycle: TaskCycle,
		retry_count: TaskRetryCount,
		phase: TaskPhase,
	) -> Self {
		Self {
			task_id,
			cycle,
			retry_count,
			phase,
		}
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for TaskExecution {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}/{}/{}", self.task_id, self.cycle, self.retry_count)
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait TaskSpawner {
	async fn block_height(&self) -> Result<u64>;

	fn execute_read(
		&self,
		target_block: u64,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: TaskCycle,
		function: Function,
		hash: String,
		block_num: u64,
	) -> Pin<Box<dyn Future<Output = Result<TssSignature>> + Send + 'static>>;

	fn execute_write(
		&self,
		function: Function,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>>;
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait TaskExecutor<B: sp_runtime::traits::Block>: Clone + Send + Sync + 'static {
	fn network(&self) -> Network;

	async fn start_tasks(
		&self,
		block_hash: B::Hash,
		block_num: u64,
		shard_id: ShardId,
	) -> Result<()>;
}

pub trait SubmitTasks<B: sp_runtime::traits::Block> {
	fn submit_task_hash(&self, block: B::Hash, shard_id: ShardId, task_id: TaskId, hash: String);
	fn submit_task_result(
		&self,
		block: B::Hash,
		task_id: TaskId,
		cycle: TaskCycle,
		status: CycleStatus,
	);
	fn submit_task_error(&self, block: B::Hash, task_id: TaskId, error: TaskError);
}
