// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod inherents;
pub mod rpc;
pub mod sharding;
pub mod slashing;

use arrayref::array_ref;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	MultiSignature,
};
use sp_std::{borrow::ToOwned, vec::Vec};

/// Time key type
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"time");

/// The type representing a signature data
// ThresholdSignature::to_bytes()
pub type SignatureData = [u8; 64];

pub type TimeSignature = MultiSignature;
pub type TimeId = <<TimeSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type TaskId = u64;

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi {
		#[allow(clippy::too_many_arguments)]
		fn store_signature(auth_key: crate::crypto::Public, auth_sig: crate::crypto::Signature, signature_data: SignatureData, event_id: ForeignEventId);
		fn get_shard_members(shard_id: u64) -> Option<Vec<TimeId>>;
		fn get_shards() -> Vec<(u64, sharding::Shard)>;
		fn report_misbehavior(shard_id: u64, offender: TimeId, reporter: TimeId, proof: crate::crypto::Signature);
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::KEY_TYPE);
}

#[derive(Debug, Eq, Copy, Clone, PartialEq, Encode, Decode, TypeInfo)]
pub struct ForeignEventId(u128);

impl ForeignEventId {
	/// Constructor, which builds proper ID from sub-ids
	/// # Param
	/// * chain_id - foreign chain ID
	/// * block_id - block of current event
	/// * event_id - ID of current event in given block
	/// * task_id - task, under which given event was created
	pub fn from_bits(chain_id: u16, block_id: u64, event_id: u16, task_id: u32) -> Self {
		let mut all = [0u8; 16];
		// chain id bits
		for (index, b) in chain_id.to_le_bytes().into_iter().enumerate() {
			all[index] = b;
		}
		// block id bits
		// it's bytes start from index 2
		for (index, b) in block_id.to_le_bytes().into_iter().enumerate() {
			all[index + 2] = b;
		}
		// event id bits
		// it's bytes start from index 10 (2 bytes of chain, 8 bytes of block)
		for (index, b) in event_id.to_le_bytes().into_iter().enumerate() {
			all[index + 10] = b;
		}
		// non_persistant_id
		// it's bytes start from index 2 (2 bytes of chain, 8 bytes of block, 2 bytes of event)
		for (index, b) in task_id.to_le_bytes().into_iter().enumerate() {
			all[index + 12] = b;
		}
		// ignoring reserved bytes for now
		Self(u128::from_le_bytes(all))
	}

	/// Returns task id from appropriate portion of bytes
	pub fn task_id(&self) -> u32 {
		let bytes = self.0.to_le_bytes();
		u32::from_le_bytes(array_ref!(bytes, 12, 4).to_owned())
	}
}

impl From<ForeignEventId> for u128 {
	fn from(val: ForeignEventId) -> Self {
		val.0
	}
}

impl From<u128> for ForeignEventId {
	fn from(source: u128) -> Self {
		Self(source)
	}
}

#[test]
fn foreign_event_id_construction_test() {
	assert_eq!(Into::<ForeignEventId>::into(0), ForeignEventId::from_bits(0, 0, 0, 0));
	assert_eq!(
		ForeignEventId::from(1208925819614629174771713),
		ForeignEventId::from_bits(1, 1, 1, 0)
	);
}
