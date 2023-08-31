// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use scale_info::prelude::string::String;
use sp_runtime::{AccountId32, MultiSignature, MultiSigner};
use sp_std::vec::Vec;

mod member;
mod shard;
mod task;

pub use crate::member::*;
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
	pub trait MembersApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>;
		fn submit_register_member(network: Network, public_key: PublicKey, peer_id: PeerId);
		fn submit_heartbeat(public_key: PublicKey);
	}

	pub trait ShardsApi {
		fn get_shards(account: &AccountId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<AccountId>;
		fn get_shard_threshold(shard_id: ShardId) -> u16;
		fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey);
	}

	pub trait TasksApi {
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution>;
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor>;
		fn submit_task_hash(shard_id: ShardId, task_id: TaskId, hash: String);
		fn submit_task_result(task_id: TaskId, cycle: TaskCycle, status: CycleStatus);
		fn submit_task_error(shard_id: ShardId, error: TaskError);
	}
}

pub trait MemberEvents {
	fn member_online(id: &AccountId);
	fn member_offline(id: &AccountId);
}

pub trait MemberStorage {
	fn member_peer_id(account: &AccountId) -> Option<PeerId>;
	fn member_public_key(account: &AccountId) -> Option<PublicKey>;
	fn is_member_online(account: &AccountId) -> bool;
}

pub trait ElectionsInterface {
	fn assign_member(member: &AccountId, network: Network);
	fn unassign_member(member: &AccountId, network: Network);
	fn random_signer(signers: Vec<AccountId>) -> PublicKey;
}

pub trait ShardsInterface {
	fn is_shard_online(shard_id: ShardId) -> bool;
	fn is_not_in_shard(account: &AccountId) -> bool;
	fn create_shard(network: Network, members: Vec<AccountId>, threshold: u16);
	fn random_signer(shard_id: ShardId) -> PublicKey;
}

pub trait TasksInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}
