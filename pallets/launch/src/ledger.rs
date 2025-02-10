use crate::airdrops::AirdropBalanceOf;
use crate::allocation::{Allocation, AllocationTracker};
use crate::deposits::BalanceOf;
use crate::stage::Stage;
use crate::{Config, Error, Event, Pallet, LAUNCH_VERSION, LOG_TARGET};

use polkadot_sdk::*;

use frame_support::ensure;
use sp_std::cmp::Ordering;

use frame_support::pallet_prelude::*;
use frame_support::traits::StorageVersion;
use sp_std::vec::Vec;

use time_primitives::{AccountId, Balance};

/// Each launch stage has a version, an issuance and the associated typed data
pub type LaunchStage = (u16, Allocation, Balance, Stage);

/// Ledger of all launch stages embedded in code.
pub type RawLaunchLedger = &'static [LaunchStage];

/// Parsed and consistency checked subsection of launch ledger
pub struct LaunchLedger<T: Config>(AllocationTracker, Vec<&'static LaunchStage>, PhantomData<T>);

impl<T: Config> LaunchLedger<T>
where
	T::AccountId: From<AccountId>,
	Balance: From<BalanceOf<T>> + From<AirdropBalanceOf<T>>,
{
	/// Parse raw launch plan and check it for consistency
	pub fn compile(ledger: RawLaunchLedger) -> Result<Self, Error<T>> {
		let mut tracker = AllocationTracker::new();
		let mut stages = Vec::<&'static LaunchStage>::new();

		// Check all stages from genesis to current launch version
		let from = Pallet::<T>::on_chain_stage_version();
		for index in 0..(LAUNCH_VERSION + 1) {
			if let Some(entry) = ledger.get(index as usize) {
				let (version, source, amount, stage) = entry;
				// Ensure the ledger is using consistent stage numbers and allocation
				ensure!(*version == index, Error::<T>::StageVersionMissmatch);
				ensure!(*source != Allocation::SIZE, Error::<T>::StageIssuanceMissmatch);

				match version.cmp(&from) {
					// Older migrations are just added up for verification
					Ordering::Less => {
						tracker.add(*source, *amount);
					},
					// Current stage is used to verify issuance.
					Ordering::Equal => {
						tracker.add(*source, *amount);
						ensure!(tracker.check::<T>(true), Error::<T>::TotalIssuanceExceeded);
					},
					// New migrations are added to launch plan
					Ordering::Greater => {
						// Ensure that stage is not retired and data is still included
						if !stage.is_executable() {
							return Err(Error::<T>::StageRetired);
						}
						// Ensure that the sum of stage data and total amount match
						if stage.sum::<T>() != *amount {
							return Err(Error::<T>::StageIssuanceMissmatch);
						}
						stages.push(entry);
					},
				}
			} else {
				// Ledger is missing a migration between genesis and current version.
				return Err(Error::<T>::StageMissing);
			}
		}

		Ok(LaunchLedger(tracker, stages, Default::default()))
	}

	/// Run a compiled and verified LaunchLedger as far as possible and return
	/// the weight spent doing so.
	pub fn run(&self) -> Weight {
		let mut weights = Weight::zero();
		let mut tracker = self.0.clone();
		for (version, source, amount, stage) in self.1.iter() {
			// This is an important invariant that we expect compile to help uphold
			debug_assert!(*version == Pallet::<T>::on_chain_stage_version() + 1);

			// Execute migration, collect weight and update on-chain stage version
			weights += stage.execute::<T>(*source);
			StorageVersion::new(*version).put::<Pallet<T>>();

			// Abort if the stage execution exceeded its allocation
			let hash = stage.hash::<T>();
			tracker.add(*source, *amount);
			if !tracker.check::<T>(false) {
				log::error!(
					target: LOG_TARGET,
					"💥 Launch stage v{} exceeded issuance: {:?}", version, hash
				);

				Pallet::<T>::deposit_event(Event::<T>::StageExceededIssuance {
					version: *version,
					hash,
				});
				break;
			}

			log::info!(
				target: LOG_TARGET,
				"🚀 Successfully executed launch stage v{}: {:?}", version, hash
			);

			// On success deposit event and continue
			Pallet::<T>::deposit_event(Event::<T>::StageExecuted { version: *version, hash });
		}
		weights
	}
}
