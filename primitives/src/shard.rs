use crate::{TaskCycle, TaskId};
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "std")]
use sp_api::ApiError;
use sp_runtime::traits::{Saturating, Zero};

pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type PeerId = [u8; 32];
pub type ShardId = u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "std", derive(Deserialize, Serialize))]
pub struct TssId(pub TaskId, pub TaskCycle);

#[cfg(feature = "std")]
impl std::fmt::Display for TssId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}/{}", self.0, self.1)
	}
}

/// Used to enforce one network per shard
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum Network {
	Ethereum,
	Astar,
}

impl core::str::FromStr for Network {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"ethereum" => Self::Ethereum,
			"astar" => Self::Astar,
			_ => anyhow::bail!("unsupported network {}", s),
		})
	}
}

/// Track status of shard
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum ShardStatus<Blocknumber> {
	Created(Blocknumber),
	Online,
	PartialOffline(u16),
	Offline,
}

impl<B: Copy> ShardStatus<B> {
	pub fn when_created(&self) -> Option<B> {
		match self {
			ShardStatus::Created(b) => Some(*b),
			_ => None,
		}
	}
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
			ShardStatus::Created(_) => ShardStatus::Offline,
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
pub trait SubmitShards<B: sp_runtime::traits::Block> {
	fn submit_tss_pub_key(
		&self,
		block: B::Hash,
		shard_id: ShardId,
		public_key: TssPublicKey,
	) -> Result<(), ApiError>;
}

#[cfg(feature = "std")]
pub struct TssRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub block_number: u64,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<([u8; 32], TssSignature)>,
}
