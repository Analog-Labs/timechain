#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub use pallet::*;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{frame_support, frame_system, sp_runtime};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::{traits::ConstU32, BoundedVec};
	use time_primitives::{AccountId, DmailMessage, DMAIL_PATH_LEN, DMAIL_TO_LEN};

	pub trait WeightInfo {
		fn send_email(to: u32, path: u32) -> Weight;
	}

	impl WeightInfo for () {
		fn send_email(_to: u32, _path: u32) -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config<AccountId = AccountId>
		+ polkadot_sdk::frame_system::Config
	{
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Message(DmailMessage),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::send_email(to.len() as u32, path.len() as u32))]
		pub fn send_email(
			origin: OriginFor<T>,
			to: BoundedVec<u8, ConstU32<DMAIL_TO_LEN>>,
			path: BoundedVec<u8, ConstU32<DMAIL_PATH_LEN>>,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let message = DmailMessage { owner, to, path };
			Self::deposit_event(Event::Message(message));
			Ok(())
		}
	}
}
