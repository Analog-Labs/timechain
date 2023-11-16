use crate::{AccountId, TaskCycle, TaskId};
#[cfg(feature = "std")]
use crate::{ApiResult, BlockHash, BlockNumber, SubmitResult};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "std", derive(Deserialize, Serialize))]
pub struct TssId(pub TaskId, pub TaskCycle);

#[cfg(feature = "std")]
impl std::fmt::Display for TssId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}/{}", self.0, self.1)
	}
}

type ChainId = u64;

/// Convert ChainId to Network
impl TryFrom<ChainId> for Network {
	type Error = ();
	fn try_from(chain_id: ChainId) -> Result<Network, ()> {
		match chain_id {
			1 => Ok(Network::Ethereum),
			592 => Ok(Network::Astar),
			137 => Ok(Network::Polygon),
			_ => Err(()),
			// 42161 => Network::Arbitrum,
			// 5 => Network::Goerli,
		}
	}
}

/// Convert Network to ChainId
impl TryFrom<Network> for ChainId {
	type Error = ();
	fn try_from(network: Network) -> Result<ChainId, ()> {
		match network {
			Network::Ethereum => Ok(1),
			Network::Astar => Ok(592),
			Network::Polygon => Ok(137),
			// Network::Arbitrum => Ok(42161),
			// Network::Goerli => Ok(5),
		}
	}
}

/// Used to enforce one network per shard
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum Network {
	Ethereum,
	Astar,
	Polygon,
}

impl core::str::FromStr for Network {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"ethereum" => Self::Ethereum,
			"astar" => Self::Astar,
			"polygon" => Self::Polygon,
			_ => anyhow::bail!("unsupported network {}", s),
		})
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
pub enum ShardStatus<Blocknumber> {
	Created(Blocknumber),
	Committed,
	Online,
	PartialOffline(u16),
	Offline,
}

impl<B> Default for ShardStatus<B> {
	fn default() -> Self {
		Self::Offline
	}
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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub struct RpcShardDetails<T> {
	state: ShardStatus<T>,
	tss_threshold: u16,
	members: Vec<(AccountId, SerializedMemberStatus)>,
	signatures: Vec<String>,
}

impl<T> RpcShardDetails<T> {
	pub fn new(
		state: ShardStatus<T>,
		tss_threshold: u16,
		members: Vec<(AccountId, SerializedMemberStatus)>,
		signatures: Vec<String>,
	) -> Self {
		Self {
			state,
			tss_threshold,
			members,
			signatures,
		}
	}
}

#[cfg(feature = "std")]
pub trait Shards {
	fn get_shards(&self, block: BlockHash, account: &AccountId) -> ApiResult<Vec<ShardId>>;

	fn get_shard_members(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<Vec<(AccountId, MemberStatus)>>;

	fn get_shard_threshold(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<u16>;

	fn get_shard_status(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<ShardStatus<BlockNumber>>;

	fn get_shard_commitment(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<Commitment>;

	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> SubmitResult;

	fn submit_online(&self, shard_id: ShardId) -> SubmitResult;
}

#[cfg(feature = "std")]
pub trait ShardsPayload {
	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> Vec<u8>;

	fn submit_online(&self, shard_id: ShardId) -> Vec<u8>;
}

#[cfg(feature = "std")]
pub struct TssSigningRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub block_number: BlockNumber,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<(TssHash, TssSignature)>,
}
