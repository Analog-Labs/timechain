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
	use polkadot_sdk::{frame_support, frame_system};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use time_primitives::{AccountId, DmailMessage, DmailPath, DmailTo};

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
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;
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
		pub fn send_email(origin: OriginFor<T>, to: DmailTo, path: DmailPath) -> DispatchResult {
			let owner = ensure_signed(origin)?;
			let message = DmailMessage { owner, to, path };
			Self::deposit_event(Event::Message(message));
			Ok(())
		}
	}
}
