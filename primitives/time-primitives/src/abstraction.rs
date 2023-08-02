use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::Serialize;
use sp_std::vec::Vec;

use crate::{crypto::Signature, sharding::Network, KeyId, ScheduleCycle, ShardId, SignatureData};
// Function defines target network endpoint
// It can be smart contract or native network API.

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EVMViewWithoutAbi { address: String, function_signature: String, input: Vec<String> },
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum ScheduleStatus {
	Initiated,
	Recurring,
	Completed,
	Invalid,
	Canceled,
}

impl ScheduleStatus {
	pub fn can_timeout(&self) -> bool {
		matches!(self, ScheduleStatus::Initiated | ScheduleStatus::Recurring)
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskSchedule<AccountId> {
	pub owner: AccountId,
	pub network: Network,
	pub function: Function,
	pub cycle: u64,
	// used to check if the task is repetitive task
	pub frequency: u64,
	pub hash: String,
	pub status: ScheduleStatus,
}

impl<AccountId> TaskSchedule<AccountId> {
	// check if task is repetitive, can't use the cycle to check because it can be decreased to 1
	pub fn is_repetitive_task(&self) -> bool {
		self.frequency > 0
	}
}
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ScheduleInput {
	pub network: Network,
	pub cycle: u64,
	pub frequency: u64,
	pub hash: String,
	pub function: Function,
}

#[derive(Encode, Decode, sp_runtime::RuntimeDebug, scale_info::TypeInfo)]
pub struct TimeTssKey {
	pub group_key: [u8; 33],
	pub shard_id: ShardId,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct OCWSigData {
	pub auth_sig: Signature,
	pub sig_data: SignatureData,
	pub key_id: KeyId,
	pub schedule_cycle: ScheduleCycle,
}

impl OCWSigData {
	pub fn new(
		auth_sig: Signature,
		sig_data: SignatureData,
		key_id: KeyId,
		schedule_cycle: ScheduleCycle,
	) -> Self {
		Self {
			auth_sig,
			sig_data,
			key_id,
			schedule_cycle,
		}
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct OCWSkdData {
	pub status: ScheduleStatus,
	pub key: KeyId,
}

impl OCWSkdData {
	pub fn new(status: ScheduleStatus, key: KeyId) -> Self {
		Self { status, key }
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
