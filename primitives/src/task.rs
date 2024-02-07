use crate::{AccountId, Balance, NetworkId, PublicKey, ShardId, TssSignature};
use codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::Percent;
use sp_std::vec::Vec;
pub type TaskId = u64;

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
	pub fn get_input_length(&self) -> u64 {
		match self {
			Function::EvmDeploy { bytecode } => bytecode.len() as u64,
			Function::EvmCall { input, .. } => input.len() as u64,
			Function::EvmViewCall { input, .. } => input.len() as u64,
			Function::EvmTxReceipt { tx } => tx.len() as u64,
			Function::RegisterShard { .. } => 0,
			Function::UnregisterShard { .. } => 0,
			Function::SendMessage { payload, .. } => payload.len() as u64,
		}
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
	pub network: NetworkId,
	pub function: Function,
	pub start: u64,
	pub shard_size: u32,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskDescriptorParams {
	pub network: NetworkId,
	pub start: u64,
	pub function: Function,
	pub funds: Balance,
	pub shard_size: u32,
}

impl TaskDescriptorParams {
	pub fn new(network: NetworkId, function: Function, shard_size: u32) -> Self {
		Self {
			network,
			start: 0,
			function,
			funds: 0,
			shard_size,
		}
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum TaskStatus {
	Created,
	Failed { error: TaskError },
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
	pub phase: TaskPhase,
}

impl TaskExecution {
	pub fn new(task_id: TaskId, phase: TaskPhase) -> Self {
		Self { task_id, phase }
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for TaskExecution {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.task_id)
	}
}

pub fn append_hash_with_task_data(data: [u8; 32], task_id: TaskId) -> Vec<u8> {
	let task_id_bytes = task_id.to_ne_bytes();
	let filler = b";";
	let mut extended_payload = Vec::with_capacity(data.len() + filler.len() + task_id_bytes.len());
	extended_payload.extend_from_slice(&data);
	extended_payload.extend_from_slice(filler);
	extended_payload.extend_from_slice(&task_id_bytes);
	extended_payload
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct DepreciationRate<BlockNumber> {
	pub blocks: BlockNumber,
	pub percent: Percent,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
/// Struct representing a task's reward configuration
/// Stored at task creation
pub struct RewardConfig<Balance, BlockNumber> {
	/// For each shard member
	pub read_task_reward: Balance,
	/// For the signer
	pub write_task_reward: Balance,
	/// For each shard member
	pub send_message_reward: Balance,
	/// Depreciation rate for all rewards
	pub depreciation_rate: DepreciationRate<BlockNumber>,
}
