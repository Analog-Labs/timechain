use crate::Config;

use scale_codec::{Decode, DecodeWithMemTracking, Encode};

use polkadot_sdk::*;

use frame_support::traits::LockIdentifier;
use sp_core::Get;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::RuntimeDebug;

#[derive(
	Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, RuntimeDebug, scale_info::TypeInfo,
)]
#[repr(u8)]
pub enum BridgedChain {
	Ethereum,
	Base,
}

impl BridgedChain {
	/// Retrieve sub id used in virtual wallet generation
	pub fn sub_id(&self) -> &'static [u8] {
		use BridgedChain::*;

		match self {
			Ethereum => b"bridged-erc20",
			Base => b"bridged-base",
		}
	}

	/// Compute account id of virtual wallet tracking issuance
	pub fn account_id<T: Config>(&self) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(self.sub_id())
	}

	/// Current identifier under which to lock tokens
	pub fn lock_id(&self) -> LockIdentifier {
		use BridgedChain::*;

		match self {
			Ethereum => *b"bridged0",
			Base => *b"bridged1",
		}
	}
}
