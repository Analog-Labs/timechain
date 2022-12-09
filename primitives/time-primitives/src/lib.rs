// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::{Block, NumberFor};
use sp_std::vec::Vec;

/// Time key type
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"time");

/// A set of runtime authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct PingGossip<B: Block> {
	/// For which block it was created
	pub block_number: NumberFor<B>,
	/// Identifier of the sender
	pub id: crypto::Public,
	/// proof of gossip's initiator
	pub signature: crypto::Signature,
}

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> Time pallet communication.
	pub trait TimeApi {
		/// Fetch current validator set from runtime
		fn get_ping_gossips(gossip_id: u64) -> Vec<crypto::Public>;
		/// Store new gossip
		fn store_ping_gossip(gossip_id: u64, acknowledged_by: crypto::Public);
		/// Gets current set of validators
		fn validator_set() -> Vec<crypto::Public>;
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::KEY_TYPE);
}
