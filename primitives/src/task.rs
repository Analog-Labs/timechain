use crate::{AccountId, Network, ScheduleCycle, ShardId, TssSignature};
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
pub struct ScheduleStatus {
	pub shard_id: ShardId,
	pub signature: TssSignature,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ScheduleError {
	pub shard_id: ShardId,
	pub error_string: String,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskSchedule {
	pub owner: AccountId,
	pub network: Network,
	pub function: Function,
	pub cycle: ScheduleCycle,
	pub start: u64,
	pub period: u64,
	pub hash: String,
}

impl TaskSchedule {
	pub fn trigger(&self, cycle: ScheduleCycle) -> u64 {
		self.start + cycle * self.period
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ScheduleInput {
	pub network: Network,
	pub cycle: ScheduleCycle,
	pub start: u64,
	pub period: u64,
	pub hash: String,
	pub function: Function,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum TaskStatus {
	Created,
	Failed { error: ScheduleError },
	Completed,
}
