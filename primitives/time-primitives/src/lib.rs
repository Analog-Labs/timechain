// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod abstraction;
pub mod sharding;

pub use abstraction::{ScheduleStatus, TaskSchedule};
use codec::Codec;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	DispatchError, MultiSignature,
};
use sp_std::vec::Vec;

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");
pub const SIG_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"psig");
pub const SKD_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"pskd");
pub const OCW_SIG_KEY: &[u8; 24] = b"pallet_sig::offchain_sig";
pub const OCW_TSS_KEY: &[u8; 24] = b"pallet_sig::offchain_tss";
pub const OCW_SKD_KEY: &[u8; 24] = b"pallet_skd::offchain_skd";

/// The type representing a signature data
// ThresholdSignature::to_bytes()
pub type SignatureData = [u8; 64];
pub type TimeSignature = MultiSignature;
pub type TimeId = <<TimeSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type TaskId = u64;
pub type KeyId = u64;
pub type ShardId = u64;
pub type ScheduleCycle = u64;

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi<AccountId, BlockNumber>
	where
	AccountId: Codec,
	BlockNumber: Codec,
	{
		fn get_shard_members(shard_id: ShardId) -> Option<Vec<TimeId>>;
		fn get_shards() -> Vec<(ShardId, sharding::Shard)>;
		fn get_active_shards(network: sharding::Network) -> Vec<(ShardId, sharding::Shard)>;
		fn get_inactive_shards(network: sharding::Network) -> Vec<(ShardId, sharding::Shard)>;
		fn get_shard_tasks(shard_id: ShardId) -> Vec<KeyId>;
		fn get_task_shard(task_id: KeyId) -> Result<ShardId, DispatchError>;
		fn get_task_schedule() -> Result<Vec<(KeyId, TaskSchedule<AccountId>)>, DispatchError>;
		fn get_task_schedule_by_key(schedule_id: KeyId) -> Result<Option<TaskSchedule<AccountId>>, DispatchError>;
		fn get_offense_count(offender: &TimeId) -> u8;
		fn get_offense_count_for_reporter(offender: &TimeId, reporter: &TimeId) -> u8;
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::TIME_KEY_TYPE);
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
