use crate::{AccountId, Network, PublicKey, ShardId, TssPublicKey, TssSignature};
#[cfg(feature = "std")]
use crate::{ApiResult, BlockHash, SubmitResult};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize, Serializer};
use sp_std::vec::Vec;
pub type TaskId = u64;
pub type TaskCycle = u64;
pub type TaskRetryCount = u8;


#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq)]
pub enum Function {
	EvmDeploy { bytecode: Vec<u8> },
	EvmCall { address: String, function_signature: String, input: Vec<String>, amount: u128 },
	EvmViewCall { address: String, function_signature: String, input: Vec<String> },
	EvmTxReceipt { tx: Vec<u8> },
	RegisterKeys {
		#[cfg_attr(feature = "std", serde(with="bytes_to_hex"))]
		key: TssPublicKey
	},
	UnregisterKeys {
		#[cfg_attr(feature = "std", serde(with="bytes_to_hex"))]
		key: TssPublicKey
	},
	SendMessage { contract_address: Vec<u8>, payload: Vec<u8> },
}

/// serde functions for handling primitive optional `u64` as [U64]
#[cfg(feature = "std")]
pub mod bytes_to_hex {
	use serde::{Deserialize, Serialize, Serializer, Deserializer};
	use const_hex::{decode, encode_prefixed, FromHexError};
	use thiserror::Error;

	#[derive(Debug, Clone, Error)]
	#[error("Failed to parse bytes: {0}")]
	pub struct ParseBytesError(FromHexError);

	pub fn serialize<S, T>(x: T, s: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
		T: AsRef<[u8]>,
	{
		s.serialize_str(&encode_prefixed(x))
	}

	pub fn deserialize<'de, const N: usize, D>(d: D) -> Result<[u8;N], D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = String::deserialize(d)?;
		let a = decode(value).map_err(serde::de::Error::custom)?;
		let mut output = [0u8;N];
		output.copy_from_slice(&a);
		Ok(output)
	}
}

impl Function {
	pub fn is_payable(&self) -> bool {
		matches!(self, Self::EvmDeploy { .. } | Self::EvmCall { .. })
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskResult {
	pub shard_id: ShardId,
	pub hash: [u8; 32],
	pub signature: TssSignature,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskError {
	pub shard_id: ShardId,
	pub msg: String,
	pub signature: TssSignature,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPhase {
	Sign,
	Write(PublicKey),
	Read(Option<Vec<u8>>),
}

impl TaskPhase {
	pub fn public_key(&self) -> Option<&PublicKey> {
		if let Self::Write(public_key) = self {
			Some(public_key)
		} else {
			None
		}
	}

	pub fn tx_hash(&self) -> Option<&[u8]> {
		if let Self::Read(Some(tx_hash)) = self {
			Some(tx_hash)
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
pub trait Tasks {
	fn get_shard_tasks(&self, block: BlockHash, shard_id: ShardId)
		-> ApiResult<Vec<TaskExecution>>;

	fn get_task(&self, block: BlockHash, task_id: TaskId) -> ApiResult<Option<TaskDescriptor>>;

	fn get_task_signature(&self, task_id: TaskId) -> ApiResult<Option<TssSignature>>;

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult;

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult;

	fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult;

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> SubmitResult;
}

#[cfg(feature = "std")]
pub trait TasksPayload {
	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> Vec<u8>;

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Vec<u8>;

	fn submit_task_result(&self, task_id: TaskId, cycle: TaskCycle, status: TaskResult) -> Vec<u8>;

	fn submit_task_error(&self, task_id: TaskId, cycle: TaskCycle, error: TaskError) -> Vec<u8>;
}

pub fn append_hash_with_task_data(
	data: [u8; 32],
	task_id: TaskId,
	task_cycle: TaskCycle,
) -> Vec<u8> {
	let task_id_bytes = task_id.to_ne_bytes();
	let task_cycle_bytes = task_cycle.to_ne_bytes();
	let filler = b";";
	let mut extended_payload = Vec::with_capacity(
		data.len() + filler.len() + task_id_bytes.len() + filler.len() + task_cycle_bytes.len(),
	);
	extended_payload.extend_from_slice(&data);
	extended_payload.extend_from_slice(filler);
	extended_payload.extend_from_slice(&task_id_bytes);
	extended_payload.extend_from_slice(filler);
	extended_payload.extend_from_slice(&task_cycle_bytes);
	extended_payload
}
