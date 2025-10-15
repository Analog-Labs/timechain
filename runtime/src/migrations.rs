use core::marker::PhantomData;

use polkadot_sdk::*;

use frame_support::traits::Get;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::traits::VestingSchedule;
use frame_support::weights::Weight;
use sp_core::hex2array;
use sp_runtime::AccountId32;

use pallet_launch::BalanceOf;

use crate::{AccountId, Balance};

const TARGET: AccountId =
	AccountId::new(hex2array!("62b615c71124915f6e840f723c68904e3cc44625a6282ee212db41f056436d41"));

pub struct ScheduleCorrectionMigration<T>(PhantomData<T>);

impl<T: pallet_vesting::Config> OnRuntimeUpgrade for ScheduleCorrectionMigration<T>
where
	T::AccountId: From<AccountId32>,
	BalanceOf<T>: From<Balance>,
{
	fn on_runtime_upgrade() -> Weight {
		let mut weight = Weight::zero();

		// Retrieve current schedule
		let schedules = pallet_vesting::Pallet::<T>::vesting(TARGET.into());
		weight += T::DbWeight::get().reads(1);

		// Check length
		let num = match schedules {
			Some(ref s) => s.len(),
			None => 0,
		};
		if num != 4 {
			log::error!("🧰 Vesting schedule does not need correction, skipping.");
			return weight;
		}

		// Find incorrect schedule
		let mut index = None;
		for (i, s) in schedules.unwrap().into_iter().enumerate() {
			if s.locked() > s.per_block() {
				if index.is_some() {
					log::error!("🧰 Duplicate in vesting schedule, aborting.");
					return weight;
				}

				index = Some(i);
			}
		}

		// Correct schedule
		if let Some(i) = index {
			weight += T::DbWeight::get().writes(1);
			if pallet_vesting::Pallet::<T>::remove_vesting_schedule(&TARGET.into(), i as u32)
				.is_err()
			{
				log::error!("🧰 Failed to correct vesting schedule, aborting.");
			}
		} else {
			log::error!("🧰 Failed to parse vesting schedule, aborting.");
		}

		weight
	}
}
