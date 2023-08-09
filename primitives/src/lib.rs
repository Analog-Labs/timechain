// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use futures::channel::oneshot;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::Serialize;
use sp_runtime::{AccountId32, DispatchResult, MultiSignature, MultiSigner};
use sp_std::vec::Vec;

mod ocw;
mod task;

pub use crate::ocw::*;
pub use crate::task::*;

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");

pub type AccountId = AccountId32;
pub type PublicKey = MultiSigner;
pub type Signature = MultiSignature;
pub type TssPublicKey = [u8; 33];
pub type TssSignature = [u8; 64];
pub type PeerId = [u8; 32];
pub type TaskId = u64;
pub type ShardId = u64;
pub type ScheduleCycle = u64;
pub type TssId = (TaskId, ScheduleCycle);

#[cfg(feature = "std")]
pub struct TssRequest {
	pub request_id: TssId,
	pub shard_id: ShardId,
	pub data: Vec<u8>,
	pub tx: oneshot::Sender<TssSignature>,
}

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi {
		fn get_shards(peer_id: PeerId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<PeerId>;
		fn get_shard_tasks(shard_id: ShardId) -> Vec<(TaskId, ScheduleCycle)>;
		fn get_task(task_id: TaskId) -> Option<TaskSchedule>;
	}
}

pub mod crypto {
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::TIME_KEY_TYPE);

	pub struct SigAuthId;

	impl frame_system::offchain::AppCrypto<crate::PublicKey, crate::Signature> for SigAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sr25519::Signature;
		type GenericPublic = sr25519::Public;
	}
}

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

pub trait ShardCreated {
	fn shard_created(shard_id: ShardId, collector: PublicKey);
}

pub trait ScheduleInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}

pub trait OcwShardInterface {
	fn benchmark_register_shard(network: Network, members: Vec<PeerId>, collector: PublicKey);
	fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey) -> DispatchResult;
	fn set_shard_offline(shard_id: ShardId, network: Network) -> DispatchResult;
}

pub trait OcwSubmitTaskResult {
	fn submit_task_result(
		task_id: TaskId,
		cycle: ScheduleCycle,
		status: ScheduleStatus,
	) -> DispatchResult;
}

pub trait ShardStatusInterface {
	fn is_shard_online(shard_id: ShardId) -> bool;
}
