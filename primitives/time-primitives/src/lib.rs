// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::Serialize;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	MultiSignature,
};
use sp_std::vec::Vec;

pub mod abstraction;

pub use crate::abstraction::*;

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");
pub const SIG_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"psig");
pub const SKD_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"pskd");
pub const OCW_TSS_KEY: &[u8; 24] = b"pallet_sig::offchain_tss";
pub const OCW_SKD_KEY: &[u8; 24] = b"pallet_skd::offchain_skd";

/// The type representing a signature data
// ThresholdSignature::to_bytes()
pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type TimeSignature = MultiSignature;
pub type TimeId = <<TimeSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type TaskId = u64;
pub type ShardId = u64;
pub type ScheduleCycle = u64;

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi {
		fn get_shards(time_id: TimeId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<TimeId>;
		fn get_shard_tasks(shard_id: ShardId) -> Vec<(TaskId, ScheduleCycle)>;
		fn get_task(task_id: TaskId) -> Option<TaskSchedule>;
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::TIME_KEY_TYPE);
}

/// Used to enforce one network per shard
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum Network {
	Ethereum,
	Astar,
}

pub trait ScheduleInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}
