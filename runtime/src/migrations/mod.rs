use polkadot_sdk::*;

use frame_support::{pallet_prelude::*, storage_alias, traits::OnRuntimeUpgrade, weights::Weight};
use sp_staking::offence::OffenceSeverity;
use sp_std::prelude::*;

// Storage alias for the old disabled validators format in the Session pallet
#[storage_alias]
type OldDisabledValidators<T: pallet_session::Config> =
	StorageValue<pallet_session::Pallet<T>, Vec<u32>, ValueQuery>;

// Storage alias for the Staking pallet's disabled validators
#[storage_alias]
type StakingDisabledValidators<T: pallet_staking::Config> =
	StorageValue<pallet_staking::Pallet<T>, Vec<(u32, OffenceSeverity)>, ValueQuery>;

/// Migration to fix the Session pallet's disabled validators and ensure compatibility
/// with the Staking pallet.
pub struct FixDisabledValidatorSet<T>(sp_std::marker::PhantomData<T>);

impl<T> OnRuntimeUpgrade for FixDisabledValidatorSet<T>
where
	T: pallet_session::Config + pallet_staking::Config,
	T::ValidatorId: From<T::AccountId> + Into<T::AccountId>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		// Save the current state for verification in post_upgrade
		let old_session_disabled = OldDisabledValidators::<T>::get();
		let staking_disabled = StakingDisabledValidators::<T>::get();

		log::info!(
			"Pre-upgrade: Session has {} disabled validators, Staking has {} disabled validators",
			old_session_disabled.len(),
			staking_disabled.len()
		);

		// Encode both for verification
		let mut encoded = Vec::new();
		old_session_disabled.encode_to(&mut encoded);
		staking_disabled.encode_to(&mut encoded);

		Ok(encoded)
	}

	fn on_runtime_upgrade() -> Weight {
		let mut weight = T::DbWeight::get().reads(2);

		// Get the current validators from the Session pallet
		let validators = pallet_session::Validators::<T>::get();
		log::info!("🔄 Session migration: Found {} validators", validators.len());

		// Get the disabled validators from both pallets
		let old_session_disabled = OldDisabledValidators::<T>::get();
		let staking_disabled = StakingDisabledValidators::<T>::get();

		// Merge the disabled validators from both sources
		let mut merged_disabled = Vec::new();

		// Add the old session disabled validators with max severity
		for validator_idx in old_session_disabled.iter() {
			if *validator_idx < validators.len() as u32 {
				merged_disabled.push((*validator_idx, OffenceSeverity::max_severity()));
			} else {
				log::warn!(
					"Ignoring out-of-bounds validator index {} (max: {})",
					validator_idx,
					validators.len() - 1
				);
			}
		}

		// Add any staking disabled validators not already in the list
		for (validator_idx, severity) in staking_disabled.iter() {
			if *validator_idx < validators.len() as u32 {
				if !merged_disabled.iter().any(|(idx, _)| idx == validator_idx) {
					merged_disabled.push((*validator_idx, *severity));
				}
			} else {
				log::warn!(
					"Ignoring out-of-bounds validator index {} (max: {})",
					validator_idx,
					validators.len() - 1
				);
			}
		}

		// Set the new disabled validators in the Session pallet
		pallet_session::DisabledValidators::<T>::put(merged_disabled.clone());
		weight = weight.saturating_add(T::DbWeight::get().writes(1));

		log::info!("✅ Session migration: Set {} disabled validators", merged_disabled.len());

		// Update the storage version to 1
		<pallet_session::Pallet<T> as OnRuntimeUpgrade>::on_runtime_upgrade();
		weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		// Decode the saved state
		let mut state_cursor = &state[..];
		let old_session_disabled = Vec::<u32>::decode(&mut state_cursor)
			.map_err(|_| "Failed to decode old session disabled validators")?;
		let staking_disabled = Vec::<(u32, OffenceSeverity)>::decode(&mut state_cursor)
			.map_err(|_| "Failed to decode staking disabled validators")?;

		// Verify the storage version is now 1
		let version = pallet_session::Pallet::<T>::on_chain_storage_version();
		frame_support::ensure!(version == 1, "Session storage version should be 1 after migration");

		// Get the current disabled validators
		let current_disabled = pallet_session::DisabledValidators::<T>::get();

		// Verify all previously disabled validators are still disabled
		for idx in old_session_disabled.iter() {
			let is_disabled = current_disabled.iter().any(|(disabled_idx, _)| disabled_idx == idx);
			// Use a static string instead of format! since DispatchError implements From<&str> but not From<String>
			frame_support::ensure!(is_disabled, "Validator should be disabled but isn't");
		}

		// Verify all staking disabled validators are included
		for (idx, _) in staking_disabled.iter() {
			let is_disabled = current_disabled.iter().any(|(disabled_idx, _)| disabled_idx == idx);
			// Use a static string instead of format! since DispatchError implements From<&str> but not From<String>
			frame_support::ensure!(
				is_disabled,
				"Validator from staking should be disabled but isn't"
			);
		}

		log::info!(
			"✅ Post-upgrade verification successful: {} disabled validators",
			current_disabled.len()
		);

		Ok(())
	}
}
