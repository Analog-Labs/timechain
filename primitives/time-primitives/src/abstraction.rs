use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::Serialize;
use sp_std::vec::Vec;

use crate::{crypto::Signature, sharding::Network, KeyId, ScheduleCycle, SignatureData, TimeId};
// Function defines target network endpoint
// It can be smart contract or native network API.

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EthereumContract {
		address: String,
		abi: String,
		function: String,
		input: Vec<Input>,
		output: Vec<Output>,
	},
	EVMViewWithoutAbi {
		address: String,
		function_signature: String,
		input: Vec<String>,
		output: Vec<Output>,
	},
	EthereumTxWithoutAbi {
		address: String,
		function_signature: String,
		input: Vec<String>,
		output: Vec<Output>,
	},
	EthereumApi {
		function: String,
		input: Vec<Input>,
		output: Vec<Output>,
	},
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Input {
	Array(Vec<Input>),
	Map(Vec<(String, (String, Input))>),
	HexAddress,
	NumberAsQuad,
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Output {
	Array(Vec<Output>),
	Skip,
	AsQuad,
	AsWord,
	AsHexString,
	AsString,
}

// Unique database identifier (it also is used as a primary key)

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct ObjectId(pub u64);

// Numeric value affinity. Where a digital point is.
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct Affinity(pub u64);

// Required value precision
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct Rounding(pub u64);

// Defines how to store collected data into collection
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Schema {
	String(String),
	Integer(String),
	Numeric(String, Option<Affinity>, Option<Rounding>),
}

impl Schema {
	pub fn name(&self) -> &str {
		match self {
			Self::String(s) | Self::Integer(s) | Self::Numeric(s, _, _) => s,
		}
	}
}

// Defines how to update collection
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct Task {
	pub task_id: ObjectId,
	pub schema: Vec<Schema>,
	pub function: Function,
	pub network: Network,
	pub with: Vec<String>,
	pub cycle: u64,
	pub validity: Validity,
	pub hash: String,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct PayableTask {
	pub task_id: ObjectId,
	pub function: Function,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum ScheduleStatus {
	Initiated,
	Recurring,
	Updated,
	Completed,
	Invalid,
	Canceled,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskSchedule<AccountId, BlockNumber> {
	pub task_id: ObjectId,
	pub owner: AccountId,
	pub network: Network,
	pub cycle: u64,
	// used to check if the task is repetitive task
	pub frequency: u64,
	pub validity: Validity,
	pub hash: String,
	pub start_execution_block: u64,
	pub executable_since: BlockNumber,
	pub status: ScheduleStatus,
}

impl<AccountId, BlockNumber> TaskSchedule<AccountId, BlockNumber> {
	// check if task is repetitive, can't use the cycle to check because it can be decreased to 1
	pub fn is_repetitive_task(&self) -> bool {
		self.frequency > 0
	}
}
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct PayableTaskSchedule<AccountId, BlockNumber> {
	pub task_id: ObjectId,
	pub owner: AccountId,
	pub network: Network,
	pub executable_since: BlockNumber,
	pub status: ScheduleStatus,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ScheduleInput {
	pub task_id: ObjectId,
	pub network: Network,
	pub cycle: u64,
	pub frequency: u64,
	pub validity: Validity,
	pub hash: String,
	pub status: ScheduleStatus,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct PayableScheduleInput {
	pub task_id: ObjectId,
	pub network: Network,
}

// Collection value
#[derive(Debug, Clone)]
pub enum Value {
	String(String),
	Numeric(String),
	Integer(String),
}

pub enum Status {
	Created(ObjectId),
	Updated(ObjectId),
	Untouched(ObjectId),
}

impl Status {
	pub fn id(&self) -> ObjectId {
		match self {
			Self::Created(id) => *id,
			Self::Updated(id) => *id,
			Self::Untouched(id) => *id,
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct QueryId(pub u64);

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub enum Validity {
	Seconds(u64),
	Cycles(u64),
	Scheduled(QueryId),
}

pub enum Data {
	Ready(Vec<Vec<(String, Value)>>),
	Scheduled(QueryId),
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct Collection {
	pub hash: String,
	pub task: Vec<u8>,
	pub validity: i64,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct EthTxValidation {
	pub blockchain: Option<String>,
	pub network: Option<String>,
	pub url: Option<String>,
	pub tx_id: String,
	pub contract_address: String,
	pub shard_id: u64,
	pub schedule_id: u64,
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
pub struct OCWReportData {
	pub shard_id: u64,
	pub offender: TimeId,
	pub proof: Signature,
}

impl OCWReportData {
	pub fn new(shard_id: u64, offender: TimeId, proof: Signature) -> Self {
		Self { shard_id, offender, proof }
	}
}
