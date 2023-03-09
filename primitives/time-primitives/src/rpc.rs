use crate::crypto::{Public, Signature};
use sp_application_crypto::RuntimeAppPublic;

/// Payload for signing through RPC
/// # Params
/// * group_id - ID of shard for signing key identification
/// * message - data to be signed
/// * signature - signature of the data done by the same Time key as current node operates used to
///   validate correctness of the data and prevent spam
pub struct SignRpcPayload {
	pub group_id: u64,
	// keccak256 hash
	pub message: [u8; 32],
	// signature is split to satisfy rpc macros and serde limits in serialization of arrays
	signature: [u8; 64],
}

impl SignRpcPayload {
	/// Constructor
	/// # Param
	/// * group_id - shard group Id
	/// * message - keccak256 hash as bytes
	/// * signature - sr25519 Signature done by TimeKey's Secret
	pub fn new(group_id: u64, message: [u8; 32], signature: [u8; 64]) -> Self {
		SignRpcPayload { group_id, message, signature }
	}

	/// Verifies this payload's signature agains given Public key
	/// # Param
	/// * key - TimeKey Public key to use as verifying key
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
