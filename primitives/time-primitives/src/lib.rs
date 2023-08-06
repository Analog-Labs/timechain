// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::Serialize;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	DispatchResult, MultiSignature,
};
use sp_std::vec::Vec;

mod ocw;
mod task;

pub use crate::ocw::*;
pub use crate::task::*;

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");

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
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, crate::TIME_KEY_TYPE);

	pub struct SigAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for SigAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for SigAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

/// Used to enforce one network per shard
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Copy, Clone, Encode, Decode, TypeInfo, PartialEq)]
pub enum Network {
	Ethereum,
	Astar,
}

pub trait ShardCreated {
	fn shard_created(shard_id: ShardId, members: sp_std::vec::Vec<TimeId>);
}

pub trait ScheduleInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}

pub trait OcwSubmitTssPublicKey {
	fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey) -> DispatchResult;
}

pub trait OcwSubmitTaskResult {
	fn submit_task_result(
		task_id: TaskId,
		cycle: ScheduleCycle,
		status: ScheduleStatus,
	) -> DispatchResult;
}
