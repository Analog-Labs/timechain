use crate::Config;

use scale_codec::{Decode, Encode};

use polkadot_sdk::*;

use frame_support::traits::LockIdentifier;
use sp_core::Get;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::RuntimeDebug;

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, scale_info::TypeInfo)]
#[repr(u8)]
pub enum Application {
	Bridging,
	OverTheCounter,
}

impl Application {
	/// Retrieve sub id used in virtual wallet generation
	pub fn sub_id(&self) -> &'static [u8] {
		use Application::*;

		match self {
			Bridging => b"bridged-erc20",
			OverTheCounter => b"over-the-counter",
		}
	}

	/// Compute account id of virtual wallet tracking issuance
	pub fn account_id<T: Config>(&self) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(self.sub_id())
	}

	/// Current identifier under which to lock tokens
	pub fn lock_id(&self) -> LockIdentifier {
		use Application::*;

		match self {
			Bridging => *b"bridged0",
			OverTheCounter => *b"otclock0",
		}
	}
}
