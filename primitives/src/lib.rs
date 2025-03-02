//! Low-level types used throughout the Substrate code.
#![cfg_attr(not(feature = "std"), no_std)]

use anyhow::Result;
use polkadot_sdk::{sp_api, sp_core, sp_runtime};
use scale_info::prelude::{string::String, vec::Vec};
use sp_core::crypto::{
	from_known_address_format, set_default_ss58_version, Ss58AddressFormatRegistry, Ss58Codec,
};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Get, IdentifyAccount, Verify},
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

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Type used to represent balances
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
/// Block header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Opaque block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Opaque block ID.
pub type BlockId = generic::BlockId<Block>;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// General Public Key used across the protocol
pub type PublicKey = MultiSigner;

/// Official timechain SS58 prefix (`an`)
#[cfg(not(any(feature = "testnet", feature = "develop")))]
pub const SS58_ADDRESS_FORMAT: Ss58AddressFormatRegistry =
	Ss58AddressFormatRegistry::AnalogTimechainAccount;

/// Unofficial testnet SS58 prefix (`at`)
#[cfg(all(feature = "testnet", not(feature = "develop")))]
pub const SS58_ADDRESS_FORMAT: Ss58AddressFormatRegistry =
	Ss58AddressFormatRegistry::AnalogTestnetAccount;

/// Unofficial develop SS58 prefix (`az`)
#[cfg(feature = "develop")]
pub const SS58_ADDRESS_FORMAT: Ss58AddressFormatRegistry =
	Ss58AddressFormatRegistry::AnalogDevelopAccount;

/// Export const primitive of raw prefifx
pub const SS58_ADDRESS_PREFIX: u16 = from_known_address_format(SS58_ADDRESS_FORMAT);

/// Helper to set default ss58 format
pub fn init_ss58_version() {
	set_default_ss58_version(SS58_ADDRESS_FORMAT.into())
}

/// Helper to format address with correct prefix
pub fn format_address(account: &AccountId) -> String {
	account.to_ss58check_with_version(SS58_ADDRESS_FORMAT.into())
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
		fn get_failed_tasks() -> Vec<TaskId>;
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
	fn shard_size(network: NetworkId) -> u16;
	fn shard_threshold(network: NetworkId) -> u16;
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
	type MaxElectionsPerBlock: Get<BlockNumber>;
	fn shard_offline(network: NetworkId, members: Vec<AccountId>);
	fn member_online(id: &AccountId, network: NetworkId);
	fn members_offline(members: Vec<AccountId>, network: NetworkId);
}

pub trait ShardsInterface {
	fn member_online(id: &AccountId, network: NetworkId);
	fn members_offline(members: Vec<AccountId>);
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_addr() {
		let addrs = [
			"5DiPYChakvifNd4oC9s5ezGYn2WebiVdf8cUXRcG1XF9Jcfm",
			"an7DqmJUeV3tjLf1NH7LRi4hu2YBftLs9K6LrAbf9UEvR8ePm",
			"atVc4Jo8T5fsFLQu4FBdwsjj6PbLVQ5ATPHoFYLpR6xyd9N1V",
			"azszGrHnFgHqmLAnkDFwU3QkHkeVJuoTmTVFev5ygjh2q9wqM",
		];
		for addr in addrs {
			let account = AccountId::from_string(addr).unwrap();
			println!("{account}");
		}
	}
}
