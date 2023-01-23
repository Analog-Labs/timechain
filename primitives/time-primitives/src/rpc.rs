use crate::crypto::{Public, Signature};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_std::vec::Vec;

/// Payload for signing through RPC
/// # Params
/// * group_id - ID of shard for signing key identification
/// * message - data to be signed
/// * signature - signature of the data done by the same Time key as current node operates used to
///   validate correctness of the data and prevent spam
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SignRpcPayload {
	pub group_id: u64,
	pub message: Vec<u8>,
	signature: Vec<u8>,
}

impl SignRpcPayload {
	pub fn new(group_id: u64, message: Vec<u8>, signature: Vec<u8>) -> Self {
		SignRpcPayload { group_id, message, signature }
	}

	pub fn verify(&self, key: Public) -> bool {
		if let Some(sig) = sp_application_crypto::sr25519::Signature::from_slice(&self.signature) {
			let signature: Signature = sig.into();
			key.verify(&self.message, &signature)
		} else {
			// Not crypto material
			false
		}
	}
}
