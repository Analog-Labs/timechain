#![cfg_attr(not(feature = "std"), no_std)]

use pallet_tesseract_sig_storage::Call;
use time_primitives::{ForeignEventId, SignatureData};
use timechain_runtime::Runtime;

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocw-");
use frame_system::offchain::{SendSignedTransaction, Signer};

pub mod crypto {
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	app_crypto!(sr25519, crate::KEY_TYPE);
}

pub fn store_signature_signed(signature_data: SignatureData, event_id: ForeignEventId) {
	let signer = Signer::<Runtime, crypto::TestAuthId>::all_accounts();
	if !signer.can_sign() {
		log::error!("No local accounts available");
		return;
	};

	let results = signer
		.send_signed_transaction(|_account| Call::store_signature { signature_data, event_id });

	for (_acc, res) in &results {
		match res {
			Ok(()) => log::info!("Event id is [{:?}]", event_id),
			Err(e) => log::error!("Failed to submit transaction with error {:?}", e),
		}
	}
}
