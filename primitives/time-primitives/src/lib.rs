// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod inherents;
pub mod rpc;
pub mod sharding;

use codec::{Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, IdentifyAccount, Verify},
	DispatchError, MultiSignature,
};
use sp_std::{fmt::Debug, vec::Vec};

/// Time key type
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"time");

/// The type representing a signature data
pub type SignatureData = Vec<u8>;

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
	/// * non_persistant_id - used if event is not included into blocks (0 otherwise)
	/// * _reserved - extra bytes for future events
	pub fn from_bits(
		chain_id: u16,
		block_id: u64,
		event_id: u16,
		non_persistant_id: u16,
		_reserved: u16,
	) -> Self {
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
		for (index, b) in non_persistant_id.to_le_bytes().into_iter().enumerate() {
			all[index + 12] = b;
		}
		// ignoring reserved bytes for now
		Self(u128::from_le_bytes(all))
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
	assert_eq!(Into::<ForeignEventId>::into(0), ForeignEventId::from_bits(0, 0, 0, 0, 0));
	assert_eq!(
		ForeignEventId::from(1208925819614629174771713),
		ForeignEventId::from_bits(1, 1, 1, 0, 0)
	);
pub trait Balance:
	AtLeast32BitUnsigned + FullCodec + Copy + Default + Debug + scale_info::TypeInfo + MaxEncodedLen
{
}
impl<
		T: AtLeast32BitUnsigned
			+ FullCodec
			+ Copy
			+ Default
			+ Debug
			+ scale_info::TypeInfo
			+ MaxEncodedLen,
	> Balance for T
{
}

pub trait WorkerTrait<AccountId, Balance> {
	fn get_reward_acc() -> Result<Vec<(AccountId, AccountId)>, DispatchError>;
	fn send_reward_to_acc(balance: Balance) -> Result<(), DispatchError>;
}
