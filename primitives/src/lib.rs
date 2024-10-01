//! Low-level types used throughout the Substrate code.
#![cfg_attr(not(feature = "std"), no_std)]

use anyhow::Result;
use async_trait::async_trait;
use frame_support::weights::Weight;
#[cfg(feature = "std")]
use futures::stream::BoxStream;
use sp_std::vec::Vec;

// Export scoped ...
pub mod currency;
pub mod dmail;
pub mod gmp;
pub mod network;
pub mod shard;
pub mod task;

// ... and unscoped
pub use crate::currency::*;
pub use crate::dmail::*;
pub use crate::gmp::*;
pub use crate::network::*;
pub use crate::shard::*;
pub use crate::task::*;

use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	DispatchResult, MultiSignature, MultiSigner, OpaqueExtrinsic,
};

/// Re-exported substrate traits
pub mod traits {
	pub use sp_core::crypto::Ss58Codec;
	pub use sp_runtime::traits::IdentifyAccount;
}

/// Re-export key and hash types
pub use sp_core::{ed25519, sr25519, H160, H256, H512};

/// Time key type identifier
pub const TIME_KEY_TYPE: sp_application_crypto::KeyTypeId =
	sp_application_crypto::KeyTypeId(*b"time");

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

pub type Balance = u128;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash to use to identify indivdual blocks.
pub type BlockHash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// General Public Key used across the protocol
pub type PublicKey = MultiSigner;

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
		fn get_heartbeat_timeout() -> BlockNumber;
		fn get_min_stake() -> Balance;
	}

	pub trait NetworksApi {
		fn get_network(network_id: NetworkId) -> Option<(ChainName, ChainNetwork)>;
	}

	pub trait ShardsApi {
		fn get_shards(account: &AccountId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)>;
		fn get_shard_threshold(shard_id: ShardId) -> u16;
		fn get_shard_status(shard_id: ShardId) -> ShardStatus;
		fn get_shard_commitment(shard_id: ShardId) -> Option<Commitment>;
	}

	pub trait TasksApi {
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution>;
		fn get_task(task_id: TaskId) -> Option<TaskDescriptor>;
		fn get_task_signature(task_id: TaskId) -> Option<TssSignature>;
		fn get_task_signer(task_id: TaskId) -> Option<PublicKey>;
		fn get_task_hash(task_id: TaskId) -> Option<[u8; 32]>;
		fn get_task_phase(task_id: TaskId) -> TaskPhase;
		fn get_task_result(task_id: TaskId) -> Option<TaskResult>;
		fn get_task_shard(task_id: TaskId) -> Option<ShardId>;
		fn get_gateway(network: NetworkId) -> Option<[u8; 20]>;
	}

	pub trait SubmitTransactionApi{
		#[allow(clippy::result_unit_err)]
		fn submit_transaction(encoded_tx: Vec<u8>) -> Result<(), ()>;
	}
}

/// Expose unbond and transfer functionality to pay for (Un)RegisterShard task fees
pub trait TransferStake {
	fn transfer_stake(from: &AccountId, to: &AccountId, amount: Balance) -> DispatchResult;
}

pub trait MemberEvents {
	fn member_online(id: &AccountId, network: NetworkId);
	fn member_offline(id: &AccountId, network: NetworkId) -> Weight;
}

pub trait MemberStorage {
	fn member_stake(account: &AccountId) -> Balance;
	fn member_peer_id(account: &AccountId) -> Option<PeerId>;
	fn member_public_key(account: &AccountId) -> Option<PublicKey>;
	fn is_member_online(account: &AccountId) -> bool;
	fn total_stake() -> Balance;
}

pub trait ElectionsInterface {
	fn shard_offline(network: NetworkId, members: Vec<AccountId>);
	fn default_shard_size() -> u16;
}

pub trait ShardsInterface {
	fn is_shard_online(shard_id: ShardId) -> bool;
	fn is_shard_member(account: &AccountId) -> bool;
	fn matching_shard_online(network: NetworkId, size: u16) -> bool;
	fn shard_members(shard_id: ShardId) -> Vec<AccountId>;
	fn shard_network(shard_id: ShardId) -> Option<NetworkId>;
	fn create_shard(network: NetworkId, members: Vec<AccountId>, threshold: u16) -> Weight;
	fn next_signer(shard_id: ShardId) -> PublicKey;
	fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey>;
}

pub trait TasksInterface {
	fn shard_online(shard_id: ShardId, network: NetworkId);
	fn shard_offline(shard_id: ShardId, network: NetworkId);
}

#[cfg(feature = "std")]
#[async_trait]
pub trait Runtime: Clone + Send + Sync + 'static {
	fn public_key(&self) -> &PublicKey;

	fn account_id(&self) -> &AccountId;

	fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)>;

	async fn get_network(&self, network: NetworkId) -> Result<Option<(String, String)>>;

	async fn get_member_peer_id(
		&self,
		block: BlockHash,
		account: &AccountId,
	) -> Result<Option<PeerId>>;

	async fn get_heartbeat_timeout(&self) -> Result<BlockNumber>;

	async fn get_min_stake(&self) -> Result<Balance>;

	async fn get_shards(&self, block: BlockHash, account: &AccountId) -> Result<Vec<ShardId>>;

	async fn get_shard_members(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<(AccountId, MemberStatus)>>;

	async fn get_shard_threshold(&self, block: BlockHash, shard_id: ShardId) -> Result<u16>;

	async fn get_shard_status(&self, block: BlockHash, shard_id: ShardId) -> Result<ShardStatus>;

	async fn get_shard_commitment(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<Option<Commitment>>;

	async fn get_shard_tasks(
		&self,
		block: BlockHash,
		shard_id: ShardId,
	) -> Result<Vec<TaskExecution>>;

	async fn get_task(&self, block: BlockHash, task_id: TaskId) -> Result<Option<TaskDescriptor>>;

	async fn get_task_signature(&self, task_id: TaskId) -> Result<Option<TssSignature>>;

	async fn get_task_signer(&self, task_id: TaskId) -> Result<Option<PublicKey>>;

	async fn get_task_hash(&self, task_id: TaskId) -> Result<Option<[u8; 32]>>;

	async fn get_gateway(&self, network: NetworkId) -> Result<Option<[u8; 20]>>;

	async fn submit_register_member(
		&self,
		network: NetworkId,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<()>;

	async fn submit_unregister_member(&self) -> Result<()>;

	async fn submit_heartbeat(&self) -> Result<()>;

	async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: ProofOfKnowledge,
	) -> Result<()>;

	async fn submit_online(&self, shard_id: ShardId) -> Result<()>;

	async fn submit_task_signature(&self, task_id: TaskId, signature: TssSignature) -> Result<()>;

	async fn submit_task_hash(&self, task_id: TaskId, hash: Result<[u8; 32], String>)
		-> Result<()>;

	async fn submit_task_result(&self, task_id: TaskId, status: TaskResult) -> Result<()>;
}
