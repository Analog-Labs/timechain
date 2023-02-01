use crate::crypto::{Public, Signature};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;

/// Payload for signing through RPC
/// # Params
/// * group_id - ID of shard for signing key identification
/// * message - data to be signed
/// * signature - signature of the data done by the same Time key as current node operates used to
///   validate correctness of the data and prevent spam
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SignRpcPayload {
	pub group_id: u64,
	pub message_a: [u8; 32],
	pub message_b: [u8; 32],
	// signature is split to satisfy rpc macros and serde limits in serialization of arrays
	signature_a: [u8; 32],
	signature_b: [u8; 32],
}

impl SignRpcPayload {
	pub fn new(
		group_id: u64,
		message_a: [u8; 32],
		message_b: [u8; 32],
		signature_a: [u8; 32],
		signature_b: [u8; 32],
	) -> Self {
		SignRpcPayload {
			group_id,
			message_a,
			message_b,
			signature_a,
			signature_b,
		}
	}

	pub fn verify(&self, key: Public) -> bool {
		let mut full_signature = self.signature_a.to_vec();
		full_signature.extend_from_slice(&self.signature_b);
		let mut full_message = self.message_a.to_vec();
		full_message.extend(self.message_b);
		if let Some(sig) = sp_application_crypto::sr25519::Signature::from_slice(&full_signature) {
			let signature: Signature = sig.into();
			key.verify(&full_message, &signature)
		} else {
			// Not crypto material
			false
		}
	}
}
