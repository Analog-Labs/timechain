#[cfg(feature = "std")]
use crate::{
	encode_gmp_events, BatchId, BlockNumber, Gateway, GatewayMessage, GmpEvent, GmpParams,
	NetworkId, TaskId,
};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedVec};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;

/// Upper bound for shard sizes
pub const MAX_SHARD_SIZE: u32 = 100;

pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type TssHash = [u8; 32];
pub type PeerId = [u8; 32];
pub type ShardId = u64;
pub type ProofOfKnowledge = [u8; 65];

#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug)]
pub struct Commitment(pub BoundedVec<TssPublicKey, ConstU32<MAX_SHARD_SIZE>>);

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

#[cfg(feature = "std")]
pub mod serde_tss_signature {
	use super::TssSignature;
	use serde::de::Error;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub fn serialize<S>(t: &TssSignature, ser: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		t[..].serialize(ser)
	}

	pub fn deserialize<'de, D>(de: D) -> Result<TssSignature, D::Error>
	where
		D: Deserializer<'de>,
	{
		let bytes = <Vec<u8>>::deserialize(de)?;
		TssSignature::try_from(bytes).map_err(|_| D::Error::custom("invalid signature length"))
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

#[cfg(feature = "std")]
impl std::fmt::Display for MemberStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let status = match self {
			Self::Added => "added",
			Self::Committed(_) => "commited",
			Self::Ready => "ready",
		};
		f.write_str(status)
	}
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
impl std::fmt::Display for ShardStatus {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let status = match self {
			Self::Created => "created",
			Self::Committed => "commited",
			Self::Online => "online",
			Self::Offline => "offline",
		};
		f.write_str(status)
	}
}

#[cfg(feature = "std")]
pub struct TssSigningRequest {
	pub task_id: TaskId,
	pub shard_id: ShardId,
	pub block: BlockNumber,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<(TssHash, TssSignature)>,
}

#[allow(clippy::result_unit_err)]
pub fn verify_signature(
	public_key: TssPublicKey,
	data: &[u8],
	signature: TssSignature,
) -> Result<(), ()> {
	let signature = schnorr_evm::Signature::from_bytes(signature).map_err(|_| ())?;
	let schnorr_public_key = schnorr_evm::VerifyingKey::from_bytes(public_key).map_err(|_| ())?;
	schnorr_public_key.verify(data, &signature).map_err(|_| ())?;
	Ok(())
}

pub struct MockTssSigner {
	signing_key: schnorr_evm::SigningKey,
}

impl MockTssSigner {
	pub fn new(shard: ShardId) -> Self {
		let shard = shard + 1;
		let mut key = [0; 32];
		key[24..32].copy_from_slice(&shard.to_be_bytes());
		Self::from_secret(key)
	}

	pub fn from_secret(secret: [u8; 32]) -> Self {
		Self {
			signing_key: schnorr_evm::SigningKey::from_bytes(secret).unwrap(),
		}
	}

	pub fn public_key(&self) -> TssPublicKey {
		self.signing_key.public().to_bytes().unwrap()
	}

	#[cfg(feature = "std")]
	pub fn sign(&self, data: &[u8]) -> TssSignature {
		self.signing_key.sign(data).to_bytes()
	}

	#[cfg(feature = "std")]
	pub fn sign_gateway_message(
		&self,
		network: NetworkId,
		gateway: Gateway,
		batch: BatchId,
		msg: &GatewayMessage,
	) -> TssSignature {
		let bytes = msg.encode(batch);
		let hash = GmpParams { network, gateway }.hash(&bytes);
		self.sign(&hash)
	}

	#[cfg(feature = "std")]
	pub fn sign_gmp_events(&self, task: TaskId, events: &[GmpEvent]) -> TssSignature {
		let bytes = encode_gmp_events(task, events);
		self.sign(&bytes)
	}
}
