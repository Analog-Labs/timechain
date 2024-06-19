#[cfg(feature = "std")]
use crate::BlockNumber;
use crate::{TaskId, TaskPhase};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use scale_codec::{Decode, Encode};
use scale_info::prelude::string::String;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;
pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type TssHash = [u8; 32];
pub type PeerId = [u8; 32];
pub type ShardId = u64;
pub type ProofOfKnowledge = [u8; 65];
pub type Commitment = Vec<TssPublicKey>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TssId {
	task_id: TaskId,
	task_phase: TaskPhase,
}

impl TssId {
	pub fn new(task_id: TaskId, task_phase: TaskPhase) -> Self {
		Self { task_id, task_phase }
	}
}

#[cfg(feature = "std")]
impl std::fmt::Display for TssId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}-{}", self.task_id, self.task_phase)
	}
}

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum MemberStatus {
	Added,
	Committed(Commitment),
	Ready,
}

impl MemberStatus {
	pub fn commitment(&self) -> Option<&Commitment> {
		if let Self::Committed(commitment) = self {
			Some(commitment)
		} else {
			None
		}
	}

	pub fn is_committed(&self) -> bool {
		self.commitment().is_some()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum SerializedMemberStatus {
	Added,
	Committed(Vec<String>),
	Ready,
}

/// Track status of shard
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum ShardStatus {
	Created,
	Committed,
	Online,
	Offline,
}

impl Default for ShardStatus {
	fn default() -> Self {
		Self::Offline
	}
}

#[cfg(feature = "std")]
pub struct TssSigningRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub block_number: BlockNumber,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<(TssHash, TssSignature)>,
}
