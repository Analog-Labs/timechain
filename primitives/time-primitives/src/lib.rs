// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;

/// Time key type
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"time");

/// The type representing a signature data
pub type SignatureData = Vec<u8>;

pub type TimeSignature = sp_application_crypto::sr25519::Signature;
pub type TimeKey = sp_application_crypto::sr25519::Public;

sp_api::decl_runtime_apis! {
	/// API necessary for Time worker <-> pallet communication.
	pub trait TimeApi {
		fn store_signature(auth_key: TimeKey, auth_sig: TimeSignature, signature_data: SignatureData, network_id: Vec<u8>, block_height: u64,);
	}
}

pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, crate::KEY_TYPE);
}
