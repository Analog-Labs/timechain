use crate::{crypto::Signature, Network, ScheduleCycle, ShardId, TaskId, TimeId, TssSignature};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::Serialize;
use sp_std::vec::Vec;

pub const OCW_MAX_TRY: u8 = 5;

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
pub enum ScheduleStatus {
	Ok(ShardId, TssSignature),
	Err(ShardId, String),
}

impl ScheduleStatus {
	pub fn shard_id(&self) -> &ShardId {
		match self {
			Self::Ok(shard_id, _) => shard_id,
			Self::Err(shard_id, _) => shard_id,
		}
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskSchedule {
	pub owner: TimeId,
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

#[derive(Encode, Decode, sp_runtime::RuntimeDebug, scale_info::TypeInfo)]
pub struct TimeTssKey {
	pub group_key: [u8; 33],
	pub shard_id: ShardId,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct OCWSkdData {
	pub task_id: TaskId,
	pub cycle: ScheduleCycle,
	pub status: ScheduleStatus,
	pub proof: Signature,
}

impl OCWSkdData {
	pub fn new(
		task_id: TaskId,
		cycle: ScheduleCycle,
		status: ScheduleStatus,
		proof: Signature,
	) -> Self {
		Self { task_id, cycle, status, proof }
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct OCWTSSGroupKeyData {
	pub shard_id: ShardId,
	pub group_key: [u8; 33],
	pub proof: Signature,
}

impl OCWTSSGroupKeyData {
	pub fn new(shard_id: ShardId, group_key: [u8; 33], proof: Signature) -> Self {
		Self { shard_id, group_key, proof }
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum OCWPayload {
	OCWSkd(OCWSkdData),
	OCWTSSGroupKey(OCWTSSGroupKeyData),
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub struct SkdMsg {
	task_id: TaskId,
	cycle: ScheduleCycle,
	status: ScheduleStatus,
}

impl SkdMsg {
	pub fn new(task_id: TaskId, cycle: ScheduleCycle, status: ScheduleStatus) -> Self {
		Self { task_id, cycle, status }
	}
}
