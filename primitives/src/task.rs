use crate::{AccountId, Network, PublicKey, ShardId, TssSignature};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;
pub type TaskId = u64;
pub type TaskCycle = u64;
pub type TaskRetryCount = u8;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	EvmDeploy { bytecode: Vec<u8> },
	EvmCall { address: [u8; 20], input: Vec<u8>, amount: u128 },
	EvmViewCall { address: [u8; 20], input: Vec<u8> },
	EvmTxReceipt { tx: Vec<u8> },
	RegisterShard { shard_id: ShardId },
	UnregisterShard { shard_id: ShardId },
	SendMessage { address: [u8; 20], payload: Vec<u8>, salt: [u8; 32], gas_limit: u64 },
}

impl Function {
	pub fn is_payable(&self) -> bool {
		matches!(self, Self::EvmDeploy { .. } | Self::EvmCall { .. })
	}

	pub fn is_gmp(&self) -> bool {
		matches!(
			self,
			Self::RegisterShard { .. } | Self::UnregisterShard { .. } | Self::SendMessage { .. }
		)
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
	pub owner: Option<AccountId>,
	pub network: Network,
	pub function: Function,
	pub cycle: TaskCycle,
	pub start: u64,
	pub period: u64,
	pub timegraph: Option<[u8; 32]>,
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
	pub timegraph: Option<[u8; 32]>,
	pub function: Function,
}

impl TaskDescriptorParams {
	pub fn new(network: Network, function: Function) -> Self {
		Self {
			network,
			cycle: 1,
			start: 0,
			period: 1,
			timegraph: None,
			function,
		}
	}
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
