// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_api::ApiError;
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
pub type BlockNumber = u32;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum TxError {
	MissingSigningKey,
	TxPoolError,
}

pub type TxResult = Result<(), TxError>;
#[cfg(feature = "std")]
pub type SubmitResult = Result<TxResult, ApiError>;

sp_api::decl_runtime_apis! {
	pub trait MembersApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>;
		fn get_heartbeat_timeout() -> u64;
		fn submit_register_member(network: Network, public_key: PublicKey, peer_id: PeerId) -> TxResult;
		fn submit_heartbeat(public_key: PublicKey) -> TxResult;
	}

	pub trait ShardsApi {
		fn get_shards(account: &AccountId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<AccountId>;
		fn get_shard_threshold(shard_id: ShardId) -> u16;
		fn get_shard_status(shard_id: ShardId) -> ShardStatus<BlockNumber>;
		fn get_shard_commitment(shard_id: ShardId) -> Commitment;
		fn submit_commitment(shard_id: ShardId, member: PublicKey, commitment: Commitment, proof_of_knowledge: ProofOfKnowledge) -> TxResult;
		fn submit_online(shard_id: ShardId, member: PublicKey) -> TxResult;
	}

	pub trait TasksApi {
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution>;
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor>;
		fn submit_task_hash(task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> TxResult;
		fn submit_task_result(task_id: TaskId, cycle: TaskCycle, status: TaskResult) -> TxResult;
		fn submit_task_error(task_id: TaskId, cycle: TaskCycle, error: TaskError) -> TxResult;
		fn submit_task_signature(task_id: TaskId, signature: TssSignature) -> TxResult;
	}

	pub trait BlockTimeApi{
		fn get_block_time_in_msec() -> u64;
	}
}

pub trait MemberEvents {
	fn member_online(id: &AccountId, network: Network);
	fn member_offline(id: &AccountId, network: Network);
}

pub trait MemberStorage {
	fn member_peer_id(account: &AccountId) -> Option<PeerId>;
	fn member_public_key(account: &AccountId) -> Option<PublicKey>;
	fn is_member_online(account: &AccountId) -> bool;
}

pub trait ElectionsInterface {
	fn shard_offline(network: Network, members: Vec<AccountId>);
}

pub trait ShardsInterface {
	fn is_shard_online(shard_id: ShardId) -> bool;
	fn is_shard_member(account: &AccountId) -> bool;
	fn create_shard(network: Network, members: Vec<AccountId>, threshold: u16);
	fn random_signer(shard_id: ShardId) -> PublicKey;
	fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey>;
}

pub trait TasksInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}
