use crate::crypto::{Public, Signature};
use sp_application_crypto::RuntimeAppPublic;
use sp_std::vec::Vec;

/// Payload for signing through RPC
/// # Params
/// * group_id - ID of shard for signing key identification
/// * message - data to be signed
/// * signature - signature of the data done by the same Time key as current node operates used to
///   validate correctness of the data and prevent spam
pub struct SignRpcPayload {
	pub group_id: u64,
	pub message: Vec<u8>,
	signature: Signature,
}

impl SignRpcPayload {
	pub fn verify(&self, key: Public) -> bool {
		key.verify(&self.message, &self.signature)
	}
}
