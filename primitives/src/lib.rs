//! Low-level types used throughout the Substrate code.
#![cfg_attr(not(feature = "std"), no_std)]

use anyhow::Result;
use polkadot_sdk::{sp_api, sp_core, sp_runtime};
use scale_info::prelude::{string::String, vec::Vec};
use sp_core::crypto::Ss58Codec;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	DispatchError, DispatchResult, MultiSignature, MultiSigner, OpaqueExtrinsic,
};

// Export scoped ...
#[cfg(feature = "std")]
pub mod admin;
#[cfg(feature = "std")]
pub mod balance;
pub mod bounds;
pub mod currency;
pub mod dmail;
pub mod gmp;
pub mod network;
pub mod shard;
pub mod task;

// ... and unscoped
pub use crate::bounds::*;
pub use crate::currency::*;
pub use crate::dmail::*;
pub use crate::gmp::*;
pub use crate::network::*;
pub use crate::shard::*;
pub use crate::task::*;

/// Re-exported substrate traits
pub mod traits {
	use polkadot_sdk::{sp_core, sp_runtime};

	pub use sp_core::crypto::Ss58Codec;
	pub use sp_runtime::traits::IdentifyAccount;
}

/// Re-export key and hash types
pub use sp_core::{ed25519, sr25519, H160, H256, H512};

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

pub const SS_58_FORMAT: u16 = 12850;

pub fn format_address(account: &AccountId) -> String {
	account.to_ss58check_with_version(sp_core::crypto::Ss58AddressFormat::custom(SS_58_FORMAT))
}

uint::construct_uint! {
	pub struct U256(4);
}

sp_api::decl_runtime_apis! {
	pub trait MembersApi {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId>;
		fn get_heartbeat_timeout() -> BlockNumber;
		fn get_min_stake() -> Balance;
	}

	pub trait NetworksApi {
		fn get_network(network_id: NetworkId) -> Option<(ChainName, ChainNetwork)>;
		fn get_gateway(network: NetworkId) -> Option<Gateway>;
	}

	pub trait ShardsApi {
		fn get_shards(account: &AccountId) -> Vec<ShardId>;
		fn get_shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)>;
		fn get_shard_threshold(shard_id: ShardId) -> u16;
		fn get_shard_status(shard_id: ShardId) -> ShardStatus;
		fn get_shard_commitment(shard_id: ShardId) -> Option<Commitment>;
	}

	pub trait TasksApi {
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskId>;
		fn get_task(task_id: TaskId) -> Option<Task>;
		fn get_task_shard(task_id: TaskId) -> Option<ShardId>;
		fn get_task_submitter(task_id: TaskId) -> Option<PublicKey>;
		fn get_task_result(task_id: TaskId) -> Option<Result<(), ErrorMsg>>;
		fn get_batch_message(batch_id: BatchId) -> Option<GatewayMessage>;
	}

	pub trait SubmitTransactionApi{
		#[allow(clippy::result_unit_err)]
		fn submit_transaction(encoded_tx: Vec<u8>) -> Result<(), ()>;
	}
}

pub trait NetworksInterface {
	fn get_networks() -> Vec<NetworkId>;
	fn gateway(network: NetworkId) -> Option<Address>;
	fn next_batch_size(network: NetworkId, block_height: u64) -> u32;
	fn batch_gas_limit(network: NetworkId) -> u128;
	fn shard_task_limit(network: NetworkId) -> u32;
}

pub trait MembersInterface {
	fn member_stake(account: &AccountId) -> Balance;
	fn member_peer_id(account: &AccountId) -> Option<PeerId>;
	fn member_public_key(account: &AccountId) -> Option<PublicKey>;
	fn is_member_registered(account: &AccountId) -> bool;
	fn is_member_online(account: &AccountId) -> bool;
	fn total_stake() -> Balance;
	fn transfer_stake(from: &AccountId, to: &AccountId, amount: Balance) -> DispatchResult;
	fn unstake_member(account: &AccountId);
}

pub trait ElectionsInterface {
	fn shard_offline(network: NetworkId, members: Vec<AccountId>);
	fn default_shard_size() -> u16;
	fn member_online(id: &AccountId, network: NetworkId);
	fn member_offline(id: &AccountId, network: NetworkId);
}

pub trait ShardsInterface {
	fn member_online(id: &AccountId, network: NetworkId);
	fn member_offline(id: &AccountId, network: NetworkId);
	fn is_shard_online(shard_id: ShardId) -> bool;
	fn is_shard_member(account: &AccountId) -> bool;
	fn shard_members(shard_id: ShardId) -> Vec<AccountId>;
	fn shard_network(shard_id: ShardId) -> Option<NetworkId>;
	fn create_shard(
		network: NetworkId,
		members: Vec<AccountId>,
		threshold: u16,
	) -> Result<ShardId, DispatchError>;
	fn next_signer(shard_id: ShardId) -> PublicKey;
	fn tss_public_key(shard_id: ShardId) -> Option<TssPublicKey>;
}

pub trait TasksInterface {
	fn shard_online(shard_id: ShardId, network: NetworkId);
	fn shard_offline(shard_id: ShardId, network: NetworkId);
	fn gateway_registered(network: NetworkId, block: u64);
}
