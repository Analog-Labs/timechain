// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use scale_info::prelude::string::String;
use sp_runtime::{AccountId32, DispatchResult, MultiSignature, MultiSigner};
use sp_std::vec::Vec;

mod ocw;
mod shard;
mod task;

pub use crate::ocw::*;
pub use crate::shard::*;
pub use crate::task::*;

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");

pub type AccountId = AccountId32;
pub type PublicKey = MultiSigner;
pub type Signature = MultiSignature;

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

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi {
		fn get_shards(peer_id: PeerId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<PeerId>;
		fn get_shard_threshold(shard_id: ShardId) -> u16;
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution>;
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor>;
		fn submit_signed(payload: OcwPayload);
		fn submit_unsigned(payload: OcwPayload);
	}
}

pub trait ShardsInterface {
	fn is_shard_online(shard_id: ShardId) -> bool;
	fn collector_pubkey(shard_id: ShardId) -> Option<PublicKey>;
	fn collector_peer_id(shard_id: ShardId) -> Option<PeerId>;
}

pub trait TasksInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}

pub trait OcwShardInterface {
	fn benchmark_register_shard(
		network: Network,
		members: Vec<PeerId>,
		collector: PublicKey,
		threshold: u16,
	);
	fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey) -> DispatchResult;
}

pub trait OcwTaskInterface {
	fn submit_task_hash(shard_id: ShardId, task_id: TaskId, hash: String) -> DispatchResult;
	fn submit_task_result(task_id: TaskId, cycle: TaskCycle, status: CycleStatus)
		-> DispatchResult;
	fn submit_task_error(task_id: TaskId, error: TaskError) -> DispatchResult;
}
