// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod abstraction;
pub mod scheduling;
pub mod sharding;

pub use abstraction::{ScheduleStatus, Task};
use codec::{Codec, Decode, Encode, FullCodec, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, IdentifyAccount, Verify},
	DispatchError, MultiSignature,
};
use sp_std::{fmt::Debug, vec::Vec};

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");
pub const SIG_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"psig");
pub const SKD_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"pskd");
pub const OCW_SIG_KEY: &[u8; 24] = b"pallet_sig::offchain_sig";
pub const OCW_REP_KEY: &[u8; 24] = b"pallet_sig::offchain_rep";
pub const OCW_TSS_KEY: &[u8; 24] = b"pallet_sig::offchain_tss";
pub const OCW_SKD_KEY: &[u8; 24] = b"pallet_skd::offchain_skd";

/// The type representing a signature data
// ThresholdSignature::to_bytes()
pub type SignatureData = [u8; 64];

pub type TimeSignature = MultiSignature;
pub type TimeId = <<TimeSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type ShardId = u64;
pub type TaskId = u64;
pub type ScheduleCycle = u64;
pub type KeyId = u64;

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi<AccountId, BlockNumber>
	where
	AccountId: Codec,
	BlockNumber: Codec,
	{
		fn get_shards(time_id: TimeId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Option<Vec<TimeId>>;
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskId>;
		fn get_task(task_id: TaskId) -> Result<Option<Task<AccountId, BlockNumber>>, DispatchError>;
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::TIME_KEY_TYPE);
}

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
		self.0 as u32

		// This is not giving right number?
		// let bytes = self.0.to_le_bytes();
		// u32::from_le_bytes(array_ref!(bytes, 12, 4).to_owned())
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
#[derive(Debug, Eq, Copy, Clone, PartialEq, Encode, Decode, TypeInfo)]
pub enum ProxyStatus {
	Valid,
	Suspended,
	Invalid,
	TokenLimitExceed,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ProxyAccStatus<AccountId, Balance> {
	pub owner: AccountId,
	pub max_token_usage: Option<Balance>,
	pub token_usage: Balance,
	pub max_task_execution: Option<u32>,
	pub task_executed: u32,
	pub status: ProxyStatus,
	pub proxy: AccountId,
}

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct ProxyAccInput<AccountId> {
	pub proxy: AccountId,
	pub max_token_usage: Option<u32>,
	pub token_usage: u32,
	pub max_task_execution: Option<u32>,
	pub task_executed: u32,
}

pub trait ProxyExtend<AccountId, Balance> {
	fn proxy_exist(acc: &AccountId) -> bool;
	fn get_master_account(acc: &AccountId) -> Option<AccountId>;
	fn proxy_update_token_used(acc: &AccountId, amount: Balance) -> bool;
}

impl<AccountId: Clone, Balance> ProxyExtend<AccountId, Balance> for () {
	fn proxy_exist(_acc: &AccountId) -> bool {
		true
	}
	fn get_master_account(acc: &AccountId) -> Option<AccountId> {
		Some(acc.clone())
	}
	fn proxy_update_token_used(_acc: &AccountId, _amount: Balance) -> bool {
		true
	}
}

pub trait PalletAccounts<AccountId> {
	fn get_treasury() -> AccountId;
}

#[test]
fn foreign_event_id_construction_test() {
	assert_eq!(Into::<ForeignEventId>::into(0), ForeignEventId::from_bits(0, 0, 0, 0));
	assert_eq!(
		ForeignEventId::from(1208925819614629174771713),
		ForeignEventId::from_bits(1, 1, 1, 0)
	);
}
