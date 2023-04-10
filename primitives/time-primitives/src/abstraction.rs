use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
use sp_std::vec::Vec;
// Function defines target network endpoint
// It can be smart contract or native network API.
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EthereumContract {
		address: String,
		abi: String,
		function: String,
		input: Vec<Input>,
		output: Vec<Output>,
	},
	EthereumApi {
		function: String,
		input: Vec<Input>,
		output: Vec<Output>,
	},
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Input {
	Array(Vec<Input>),
	Map(Vec<(String, (String, Input))>),
	HexAddress,
	NumberAsQuad,
}

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
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct ObjectId(pub u64);

// Numeric value affinity. Where a digital point is.
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct Affinity(pub u64);

// Required value precision
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct Rounding(pub u64);

// Defines how to store collected data into collection
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
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct Task {
	pub collection_id: ObjectId,
	pub schema: Vec<Schema>,
	pub function: Function,
	pub with: Vec<String>,
	pub cycle: u64,
	pub validity: Validity,
	pub hash: String,
}
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum ScheduleStatus {
	Initiated,
	Updated,
	Completed,
	Canceled,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskSchedule<AccountId> {
	pub task_id: ObjectId,
	pub owner: AccountId,
	pub shard_id: u64,
	pub cycle: u64,
	pub validity: Validity,
	pub hash: String,
	pub status: ScheduleStatus,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ScheduleInput {
	pub task_id: ObjectId,
	pub shard_id: u64,
	pub cycle: u64,
	pub validity: Validity,
	pub hash: String,
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

#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq)]
pub struct QueryId(pub u64);
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
