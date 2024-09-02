#[cfg(feature = "std")]
use crate::BlockNumber;
#[cfg(feature = "std")]
use futures::channel::oneshot;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use crate::TaskExecution;
use scale_codec::{Decode, Encode};
use scale_info::prelude::string::String;
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;

pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type TssHash = [u8; 32];
pub type PeerId = [u8; 32];
pub type ShardId = u64;
pub type ProofOfKnowledge = [u8; 65];
pub type Commitment = Vec<TssPublicKey>;

#[cfg(feature = "std")]
pub mod serde_tss_public_key {
	use super::TssPublicKey;
	use serde::de::Error;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub fn serialize<S>(t: &TssPublicKey, ser: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		t[..].serialize(ser)
	}

	pub fn deserialize<'de, D>(de: D) -> Result<TssPublicKey, D::Error>
	where
		D: Deserializer<'de>,
	{
		let bytes = <Vec<u8>>::deserialize(de)?;
		TssPublicKey::try_from(bytes).map_err(|_| D::Error::custom("invalid public key length"))
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
	Offline = 4, // To remove the "= 4", please write a migration!
}

impl Default for ShardStatus {
	fn default() -> Self {
		Self::Offline
	}
}

#[cfg(feature = "std")]
pub struct TssSigningRequest {
	pub request_id: TaskExecution,
	pub shard_id: ShardId,
	pub block_number: BlockNumber,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<(TssHash, TssSignature)>,
}
