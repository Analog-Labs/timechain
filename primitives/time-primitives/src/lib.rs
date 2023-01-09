// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod inherents;

use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	MultiSignature,
};
use sp_std::vec::Vec;

/// Time key type
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"time");

/// The type representing a signature data
pub type SignatureData = Vec<u8>;

pub type TimeSignature = MultiSignature;
pub type TimeId = <<TimeSignature as Verify>::Signer as IdentifyAccount>::AccountId;

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi {
		#[allow(clippy::too_many_arguments)]
		fn store_signature(auth_key: TimeId, auth_sig: TimeSignature, signature_data: SignatureData, task_id: u64, block_height: u64,);
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::KEY_TYPE);
}
