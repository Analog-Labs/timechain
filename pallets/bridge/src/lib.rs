#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
//! # Analog Timebridge Pallet
//!
//! Simple centralized bridge implementation

pub use pallet::*;

use polkadot_sdk::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod benchmarks;

mod bridged;

use bridged::BridgedChain;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{
		Currency, ExistenceRequirement, LockableCurrency, WithdrawReasons,
	};
	use frame_support::PalletId;
	use frame_system::pallet_prelude::*;

	pub trait WeightInfo {
		fn bridge() -> Weight;
	}

	pub struct TestWeightInfo;
	impl WeightInfo for TestWeightInfo {
		fn bridge() -> Weight {
			Weight::zero()
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_balances::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// Identifier to use for pallet-owned wallets
		type PalletId: Get<PalletId>;
		/// Weight information of the pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A bridging request has been issued.
		BridgeRequest { chain: BridgedChain, address: [u8; 20], amount: T::Balance },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer and lock funds to trigger bridge request event.
		///
		/// Used for minting funds on other chains in a manual one-way bridge.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::bridge())]
		pub fn bridge(
			origin: OriginFor<T>,
			chain: BridgedChain,
			address: [u8; 20],
			amount: T::Balance,
		) -> DispatchResult {
			let source = ensure_signed(origin)?;
			let destination = chain.account_id::<T>();

			pallet_balances::Pallet::<T>::transfer(
				&source,
				&destination,
				amount,
				ExistenceRequirement::AllowDeath,
			)?;
			pallet_balances::Pallet::<T>::set_lock(
				chain.lock_id(),
				&destination,
				pallet_balances::Pallet::<T>::total_balance(&destination),
				WithdrawReasons::all(),
			);

			Pallet::<T>::deposit_event(Event::<T>::BridgeRequest { chain, address, amount });

			Ok(())
		}
	}
}
