#![cfg_attr(not(feature = "std"), no_std)]
//! # Analog Governance Pallet
//!
//! This is custom pallet to be used for the ever extending mainnet governance.
//!
//! Currently only wraps a few important root call to lower the required privilege level
//! to a custom origin.
//!
//! See [`Calls`] for a list of wrapped extrinsics.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use polkadot_sdk::{frame_support, frame_system};

	// Additional custom imports
	use frame_system::{RawOrigin, WeightInfo as SystemWeights};

	use pallet_staking::{ConfigOp, WeightInfo as StakingWeights};
	use polkadot_sdk::{pallet_balances, pallet_staking, sp_runtime};
	use sp_runtime::{Perbill, Percent};

	// Useful coupling shorthands
	type CurrencyBalanceOf<T> = <T as pallet_staking::Config>::CurrencyBalance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_balances::Config + pallet_staking::Config
	{
		/// Allowed origin for system calls
		type SystemAdmin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Allowed origin for staking calls
		type StakingAdmin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// Wrapper around system pallet calls
		#[pallet::call_index(0)]
		#[pallet::weight(<T as frame_system::Config>::SystemWeightInfo::authorize_upgrade())]
		pub fn authorize_upgrade(origin: OriginFor<T>, code_hash: T::Hash) -> DispatchResult {
			T::SystemAdmin::ensure_origin(origin)?;
			frame_system::Pallet::<T>::authorize_upgrade(RawOrigin::Root.into(), code_hash)
		}

		// Wrapper around staking pallet calls
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_validator_count())]
		pub fn set_validator_count(
			origin: OriginFor<T>,
			#[pallet::compact] new: u32,
		) -> DispatchResult {
			T::StakingAdmin::ensure_origin(origin)?;
			pallet_staking::Pallet::<T>::set_validator_count(RawOrigin::Root.into(), new)
		}

		#[allow(clippy::too_many_arguments)]
		#[pallet::call_index(2)]
		#[pallet::weight(
			<T as pallet_staking::Config>::WeightInfo::set_staking_configs_all_set()
                .max(<T as pallet_staking::Config>::WeightInfo::set_staking_configs_all_remove())
        )]
		pub fn set_staking_configs(
			origin: OriginFor<T>,
			min_nominator_bond: ConfigOp<CurrencyBalanceOf<T>>,
			min_validator_bond: ConfigOp<CurrencyBalanceOf<T>>,
			max_nominator_count: ConfigOp<u32>,
			max_validator_count: ConfigOp<u32>,
			chill_threshold: ConfigOp<Percent>,
			min_commission: ConfigOp<Perbill>,
			max_staked_rewards: ConfigOp<Percent>,
		) -> DispatchResult {
			T::StakingAdmin::ensure_origin(origin)?;
			pallet_staking::Pallet::<T>::set_staking_configs(
				RawOrigin::Root.into(),
				min_nominator_bond,
				min_validator_bond,
				max_nominator_count,
				max_validator_count,
				chill_threshold,
				min_commission,
				max_staked_rewards,
			)
		}
	}
}
