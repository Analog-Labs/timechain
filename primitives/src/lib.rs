// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use futures::stream::BoxStream;
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

#[cfg(feature = "std")]
pub trait Runtime: Clone + Send + Sync + 'static {
	fn get_block_time_in_ms(&self) -> ApiResult<u64>;

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)>;

	fn public_key(&self) -> PublicKey;

	fn account_id(&self) -> AccountId;

	fn get_network(&self, network: NetworkId) -> ApiResult<Option<(String, String)>>;

	fn get_member_peer_id(
		&self,
		block: BlockHash,
		account: &AccountId,
	) -> ApiResult<Option<PeerId>>;

	fn get_heartbeat_timeout(&self) -> ApiResult<u64>;

	fn get_min_stake(&self) -> ApiResult<u128>;

	fn submit_register_member(
		&self,
		network: Network,
		peer_id: PeerId,
		stake_amount: u128,
	) -> SubmitResult;

	fn submit_heartbeat(&self) -> SubmitResult;

	fn get_shards(&self, block: BlockHash, account: &AccountId) -> ApiResult<Vec<ShardId>>;

	fn get_shard_members(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<Vec<(AccountId, MemberStatus)>>;

	fn get_shard_threshold(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<u16>;

	fn get_shard_status(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> ApiResult<ShardStatus<BlockNumber>>;

	fn get_shard_commitment(&self, block: BlockHash, shard_id: ShardId) -> ApiResult<Commitment>;

	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> SubmitResult;

	fn submit_online(&self, shard_id: ShardId) -> SubmitResult;

	fn get_shard_tasks(&self, block: BlockHash, shard_id: ShardId)
		-> ApiResult<Vec<TaskExecution>>;

	fn get_task(&self, block: BlockHash, task_id: TaskId) -> ApiResult<Option<TaskDescriptor>>;

	fn get_task_signature(&self, task_id: TaskId) -> ApiResult<Option<TssSignature>>;

	fn get_gateway(&self, network: Network) -> ApiResult<Option<Vec<u8>>>;

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> SubmitResult;

	fn submit_task_result(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		status: TaskResult,
	) -> SubmitResult;

	fn submit_task_error(
		&self,
		task_id: TaskId,
		cycle: TaskCycle,
		error: TaskError,
	) -> SubmitResult;

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> SubmitResult;
}

#[cfg(feature = "std")]
pub trait TxBuilder {
	fn submit_register_member(
		&self,
		network: Network,
		public_key: PublicKey,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Vec<u8>;
	fn submit_heartbeat(&self) -> Vec<u8>;

	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> Vec<u8>;

	fn submit_online(&self, shard_id: ShardId) -> Vec<u8>;

	fn submit_task_hash(&self, task_id: TaskId, cycle: TaskCycle, hash: Vec<u8>) -> Vec<u8>;

	fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Vec<u8>;

	fn submit_task_result(&self, task_id: TaskId, cycle: TaskCycle, status: TaskResult) -> Vec<u8>;

	fn submit_task_error(&self, task_id: TaskId, cycle: TaskCycle, error: TaskError) -> Vec<u8>;
}
