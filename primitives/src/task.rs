use crate::{AccountId, Network, TaskCycle, ShardId, TssSignature};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::Serialize;
use sp_std::vec::Vec;

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EVMViewWithoutAbi { address: String, function_signature: String, input: Vec<String> },
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum FunctionResult {
	EVMViewWithoutAbi { result: Vec<String> },
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
	Completed,
}
