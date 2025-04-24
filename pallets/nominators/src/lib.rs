#![cfg_attr(not(feature = "std"), no_std)]
//#![allow(clippy::manual_inspect)]
//! # Analog Nominator Pallet
//!
//! Custom pallet handles automatic nomination rotation.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Type used for unique identifier of each stash.
pub type StashId = u32;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;

	use polkadot_sdk::{frame_support, frame_system};
	use polkadot_sdk::{pallet_staking, sp_runtime};

	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::PalletId;

	// Weight structures
	pub trait WeightInfo {
		fn add_whitelisted() -> Weight;
		fn remove_whitelisted() -> Weight;
	}

	pub struct TestWeightInfo;
	impl WeightInfo for TestWeightInfo {
		fn add_whitelisted() -> Weight {
			Weight::zero()
		}
		fn remove_whitelisted() -> Weight {
			Weight::zero()
		}
	}

	// Useful coupling shorthands
	type CurrencyBalanceOf<T> = <T as pallet_staking::Config>::CurrencyBalance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_staking::Config {
		/// Identifier to use for pallet-owned wallets
		type PalletId: Get<PalletId>;
		/// Allowed origin for system calls
		type WhitelistAdmin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Call weight benchmark
		type WeightInfo: WeightInfo;
	}

	/// List of accounts to nominate
	#[pallet::storage]
	pub type Whitelist<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			//let next_election = T::DataProvider::next_election_prediction(now).max(now);
			//let deadline = T::SignedPhase::get() + T::UnsignedPhase::get();

			Weight::zero()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::add_whitelisted())]
		pub fn add_whitelisted(origin: OriginFor<T>, validator: T::AccountId) -> DispatchResult {
			T::WhitelistAdmin::ensure_origin(origin)?;

			Whitelist::<T>::insert(validator, ());

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_whitelisted())]
		pub fn remove_whitelisted(origin: OriginFor<T>, validator: T::AccountId) -> DispatchResult {
			T::WhitelistAdmin::ensure_origin(origin)?;

			Whitelist::<T>::remove(validator);

			Ok(())
		}
	}
}
