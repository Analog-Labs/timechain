#[cfg(feature = "std")]
use crate::BlockNumber;
use crate::TaskId;
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use scale_info::prelude::string::String;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::traits::{Saturating, Zero};
use sp_std::vec::Vec;
pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type TssHash = [u8; 32];
pub type PeerId = [u8; 32];
pub type ShardId = u64;
pub type ProofOfKnowledge = [u8; 65];
pub type Commitment = Vec<TssPublicKey>;
pub type TssId = TaskId;

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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum ShardPulse {
	Online,
	PartialOffline(u16),
	Offline,
}

impl Default for ShardPulse {
	fn default() -> ShardPulse {
		ShardPulse::Offline
	}
}

/// Track status of shard
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum ShardStatus {
	Created,
	Committed,
}

// TODO: remove
impl Default for ShardStatus {
	fn default() -> ShardStatus {
		ShardStatus::Created
	}
}

impl ShardStatus {
	pub fn online_member(&self) -> Self {
		match self {
			ShardStatus::PartialOffline(count) => {
				let new_count = count.saturating_less_one();
				if new_count.is_zero() {
					ShardStatus::Online
				} else {
					ShardStatus::PartialOffline(new_count)
				}
			},
			_ => *self,
		}
	}

	pub fn offline_member(&self, max: u16) -> Self {
		match self {
			ShardStatus::PartialOffline(count) => {
				let new_count = count.saturating_plus_one();
				if new_count > max {
					ShardStatus::Offline
				} else {
					ShardStatus::PartialOffline(new_count)
				}
			},
			// if a member goes offline before the group key is submitted,
			// then the shard will never go online
			// TODO: add check to ShardStatus
			// ShardStatus::Created(_) => ShardStatus::Offline,
			ShardStatus::Online => {
				if max.is_zero() {
					ShardStatus::Offline
				} else {
					ShardStatus::PartialOffline(1)
				}
			},
			_ => *self,
		}
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
