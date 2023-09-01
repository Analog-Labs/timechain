use crate::{TaskCycle, TaskId, TaskRetryCount};
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::Serialize;
use sp_std::vec::Vec;

pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type PeerId = [u8; 32];
pub type ShardId = u64;
pub type TssId = (TaskId, TaskCycle, TaskRetryCount);

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
pub enum ShardStatus {
	Created,
	Online,
	Offline,
}

#[cfg(feature = "std")]
pub struct TssRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<TssSignature>,
}
