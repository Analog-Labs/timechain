use crate::{AccountId, Balance, NetworkId, ShardId, TssSignature};
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
	// sign
	RegisterShard { shard_id: ShardId },
	UnregisterShard { shard_id: ShardId },
	SendMessage { msg: Msg },
	// write
	EvmDeploy { bytecode: Vec<u8> },
	EvmCall { address: [u8; 20], input: Vec<u8>, amount: u128 },
	// read
	EvmViewCall { address: [u8; 20], input: Vec<u8> },
	EvmTxReceipt { tx: [u8; 32] },
	ReadMessages,
}

impl Function {
	pub fn initial_phase(&self) -> TaskPhase {
		match self {
			Self::RegisterShard { .. }
			| Self::UnregisterShard { .. }
			| Self::SendMessage { .. } => TaskPhase::Sign,
			Self::EvmDeploy { .. } | Self::EvmCall { .. } => TaskPhase::Write,
			Self::EvmViewCall { .. } | Self::EvmTxReceipt { .. } | Self::ReadMessages => {
				TaskPhase::Read
			},
		}
	}

	pub fn get_input_length(&self) -> u64 {
		self.encoded_size() as _
	}
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskResult {
	pub shard_id: ShardId,
	pub payload: Payload,
	pub signature: TssSignature,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Payload {
	Hashed([u8; 32]),
	Error(String),
	Gmp(Vec<Msg>),
}

impl Payload {
	pub fn bytes(&self, task_id: TaskId) -> Vec<u8> {
		(task_id, self).encode()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, Decode, Encode, TypeInfo, PartialEq)]
pub struct Msg {
	pub source_network: NetworkId,
	pub source: [u8; 32],
	pub dest_network: NetworkId,
	pub dest: [u8; 20],
	pub gas_limit: u128,
	pub salt: [u8; 32],
	pub data: Vec<u8>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskDescriptor {
	pub owner: Option<AccountId>,
	pub network: NetworkId,
	pub function: Function,
	pub start: u64,
	pub shard_size: u16,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskDescriptorParams {
	pub network: NetworkId,
	pub start: u64,
	pub function: Function,
	pub funds: Balance,
	pub shard_size: u16,
}

impl TaskDescriptorParams {
	pub fn new(network: NetworkId, function: Function, shard_size: u16) -> Self {
		Self {
			network,
			start: 0,
			function,
			funds: 0,
			shard_size,
		}
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Decode, Encode, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPhase {
	Sign,
	Write,
	Read,
}

impl Default for TaskPhase {
	fn default() -> Self {
		Self::Read
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

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskFunder {
	Account(AccountId),
	Shard(ShardId),
}
