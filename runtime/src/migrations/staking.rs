use frame_support::{
	migrations::VersionedMigration,
	pallet_prelude::*,
	traits::{Get, UncheckedOnRuntimeUpgrade},
};
use frame_system::RawOrigin;
use pallet_staking::{Pallet, StakingLedger};
use polkadot_sdk::*;
use sp_runtime::traits::Zero;
use sp_std::vec::Vec;

#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

/// A migration that:
/// 1. Increments the storage version of the staking pallet
/// 2. Migrates all nominators with stake in T::OldCurrency to the new currency system
pub struct VersionUncheckedMigrateStakingCurrency<T>(sp_std::marker::PhantomData<T>);

impl<T: pallet_staking::Config> UncheckedOnRuntimeUpgrade
	for VersionUncheckedMigrateStakingCurrency<T>
{
	fn on_runtime_upgrade() -> Weight {
		let mut weight = T::DbWeight::get().reads(1);

		// Collect all stashes that need migration
		let stashes_to_migrate = Self::collect_stashes_with_old_currency();
		let count = stashes_to_migrate.len() as u64;

		// Migrate each stash
		for stash in stashes_to_migrate {
			if let Err(err) = <Pallet<T>>::migrate_currency(RawOrigin::Root.into(), stash.clone()) {
				log::warn!(
					target: "runtime::staking",
					"Failed to migrate currency for stash {:?}: {:?}",
					stash, err
				);
			}

			// Update weight for each migration
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(5, 3));
		}

		log::info!(
			target: "runtime::staking",
			"Staking currency migration completed for {} stashes",
			count
		);

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let stashes_to_migrate = Self::collect_stashes_with_old_currency();

		log::info!(
			target: "runtime::staking",
			"Staking currency migration: found {} stashes to migrate",
			stashes_to_migrate.len()
		);

		// Save the number of stashes to migrate for post-upgrade verification
		Ok(stashes_to_migrate.len().to_le_bytes().to_vec())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let expected_count = if !state.is_empty() {
			let mut bytes = [0u8; 8];
			bytes.copy_from_slice(&state[0..8]);
			usize::from_le_bytes(bytes)
		} else {
			0
		};

		let stashes_with_old_currency = Self::collect_stashes_with_old_currency();

		log::info!(
			target: "runtime::staking",
			"Staking currency migration: {} stashes still with old currency (expected to migrate {})",
			stashes_with_old_currency.len(),
			expected_count
		);

		// Verify that most stashes were migrated
		frame_support::ensure!(
			stashes_with_old_currency.len() < expected_count,
			"Not enough stashes were migrated"
		);

		Ok(())
	}
}

impl<T: pallet_staking::Config> VersionUncheckedMigrateStakingCurrency<T> {
	/// Collects all stashes that have stake in T::OldCurrency
	fn collect_stashes_with_old_currency() -> Vec<T::AccountId> {
		let mut stashes = Vec::new();

		// Iterate through all ledgers to find those using old currency
		pallet_staking::Ledger::<T>::iter().for_each(|(stash, ledger)| {
			// Check if the stash is using old currency
			if Self::is_using_old_currency(&stash, &ledger) {
				stashes.push(stash);
			}
		});

		stashes
	}

	/// Checks if a stash is using the old currency
	fn is_using_old_currency(stash: &T::AccountId, ledger: &StakingLedger<T>) -> bool {
		// Check if the stash has a balance lock but no asset hold
		let has_lock = !ledger.total.is_zero();
		let has_asset = pallet_staking::asset::staked::<T>(stash).is_zero();

		has_lock && has_asset
	}
}

/// The final migration type that can be used in the runtime.
/// This will automatically handle the storage version checking and incrementing.
pub type StakingCurrencyMigration<T> = VersionedMigration<
	15, // Current storage version
	16, // Target storage version
	VersionUncheckedMigrateStakingCurrency<T>,
	Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;
