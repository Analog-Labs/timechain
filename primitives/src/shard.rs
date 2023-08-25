use crate::{TaskCycle, TaskId};
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

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

/// Track status of shard
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum ShardStatus<Blocknumber> {
	Created(Blocknumber),
	Online,
	Offline,
}

impl<B: Copy> ShardStatus<B> {
	pub fn when_created(&self) -> Option<B> {
		match self {
			ShardStatus::Created(b) => Some(*b),
			_ => None,
		}
	}
}

#[cfg(feature = "std")]
pub struct TssRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub block_number: u64,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<TssSignature>,
}
