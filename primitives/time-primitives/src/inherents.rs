use crate::sharding::{ShardId, ShardPublicKey};
use codec::{Decode, Encode};
use sp_inherents::{InherentIdentifier, IsFatalError};

/// ID of inherent data we submit to runtime
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"tsskey01";
/// TSS Public key output type
#[derive(Encode, Decode, sp_runtime::RuntimeDebug, scale_info::TypeInfo)]
pub struct TimeTssKey {
	pub shard_id: ShardId,
	pub group_key: ShardPublicKey,
}

/// Errors that can occur while checking the Time inherent.
#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// This is a fatal-error and will stop block import.
	#[cfg_attr(feature = "std", error("The inserted group public key is invalid."))]
	InvalidGroupKey(TimeTssKey),
	/// This is a fatal-error and will stop block import.
	#[cfg_attr(feature = "std", error("Wrong Inherent Call in Block"))]
	WrongInherentCall,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		match self {
			InherentError::InvalidGroupKey(_) => true,
			InherentError::WrongInherentCall => true,
		}
	}
}

impl InherentError {
	/// Try to create an instance ouf of the given identifier and data.
	#[cfg(feature = "std")]
	pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
		if id == &INHERENT_IDENTIFIER {
			<InherentError as codec::Decode>::decode(&mut &data[..]).ok()
		} else {
			None
		}
	}
}
