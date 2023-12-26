// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_api::ApiError;
use sp_runtime::{AccountId32, MultiSignature, MultiSigner};
use sp_std::vec::Vec;

mod member;
mod network;
mod shard;
mod task;

pub use crate::member::*;
pub use crate::network::*;
pub use crate::shard::*;
pub use crate::task::*;

/// Time key type
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");
pub const TSS_KEY_PATH: &str = "analog";

pub type AccountId = AccountId32;
pub type PublicKey = MultiSigner;
pub type Signature = MultiSignature;
pub type BlockNumber = u32;
pub type BlockHash = sp_core::H256;

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
#[cfg(feature = "std")]
pub type ApiResult<T> = Result<T, ApiError>;

sp_api::decl_runtime_apis! {
	pub trait MembersApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>;
		fn get_heartbeat_timeout() -> u64;
		fn get_min_stake() -> u128;
	}

	pub trait NetworksApi {
		fn get_network(network_id: NetworkId) -> Option<(ChainName, ChainNetwork)>;
		fn get_chain_id(network_id: NetworkId) -> Option<ChainId>;
	}

	pub trait ShardsApi {
		fn get_shards(account: &AccountId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)>;
		fn get_shard_threshold(shard_id: ShardId) -> u16;
		fn get_shard_status(shard_id: ShardId) -> ShardStatus<BlockNumber>;
		fn get_shard_commitment(shard_id: ShardId) -> Commitment;
	}

	pub trait TasksApi {
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution>;
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor>;
		fn get_task_signature(task_id: TaskId) -> Option<TssSignature>;
		fn get_task_cycle(task_id: TaskId) -> TaskCycle;
		fn get_task_phase(task_id: TaskId) -> TaskPhase;
		fn get_task_results(task_id: TaskId, cycle: Option<TaskCycle>) -> Vec<(TaskCycle, TaskResult)>;
		fn get_task_shard(task_id: TaskId) -> Option<ShardId>;
		fn get_gateway(network: Network) -> Option<Vec<u8>>;
	}

	pub trait BlockTimeApi{
		fn get_block_time_in_msec() -> u64;
	}

	pub trait SubmitTransactionApi{
		fn submit_transaction(encoded_tx: Vec<u8>) -> TxResult;
	}
}

pub trait MemberEvents {
	fn member_online(id: &AccountId, network: Network);
	fn member_offline(id: &AccountId, network: Network);
}

pub trait MemberStorage {
	type Balance: Ord;
	fn member_stake(account: &AccountId) -> Self::Balance;
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
	fn shard_network(shard_id: ShardId) -> Option<Network>;
	fn create_shard(network: Network, members: Vec<AccountId>, threshold: u16);
	fn random_signer(shard_id: ShardId) -> PublicKey;
	fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey>;
}

pub trait TasksInterface {
	fn shard_online(shard_id: ShardId, network: Network);
	fn shard_offline(shard_id: ShardId, network: Network);
}
