use core::marker::PhantomData;

use polkadot_sdk::*;

use frame_support::traits::Get;
use frame_support::traits::OnRuntimeUpgrade;
use frame_support::traits::VestingSchedule;
use frame_support::weights::Weight;
use sp_core::hex2array;
use sp_runtime::AccountId32;

use pallet_launch::BalanceOf;

use crate::{AccountId, Balance, BlockNumber, ANLOG};

const TARGET: AccountId =
	AccountId::new(hex2array!("94f4137957e30fe279ad8f30dc7a4fd6c5dabb8b8ff3286484c66cbac68ac4f2"));

const UNLOCKS: [BlockNumber; 3] = [
	// 5 + 6 Months
	5_522_070, // 5 + 12 Months
	8_157_270, // 5 + 18 Months
	10_792_470,
];

const QUARTER: Balance = 18_115_950 * ANLOG;

pub struct ScheduleCorrectionMigration<T>(PhantomData<T>);

impl<T: pallet_vesting::Config> OnRuntimeUpgrade for ScheduleCorrectionMigration<T>
where
	T::AccountId: From<AccountId32>,
	BalanceOf<T>: From<Balance>,
{
	fn on_runtime_upgrade() -> Weight {
		//let account = AccountId::new(TARGET);
		let mut weight = Weight::zero();

		let num = match pallet_vesting::Pallet::<T>::vesting(TARGET.into()) {
			Some(scheds) => scheds.len(),
			None => 0,
		};
		weight += T::DbWeight::get().reads(1);

		if num == 0 {
			log::error!("🧰 Failed to detect vesting schedule, aborting.");
			return weight;
		} else if num > 1 {
			log::error!("🧰 Multiple vesting schedules detected, aborting.");
			return weight;
		}

		weight += T::DbWeight::get().writes(1);
		if pallet_vesting::Pallet::<T>::remove_vesting_schedule(&TARGET.into(), 0).is_err() {
			log::error!("🧰 Failed to remove vesting schedule, aborting.");
			return weight;
		}

		for schedule in UNLOCKS {
			weight += T::DbWeight::get().writes(1);
			if pallet_vesting::Pallet::<T>::add_vesting_schedule(
				&TARGET.into(),
				QUARTER.into(),
				QUARTER.into(),
				schedule.into(),
			)
			.is_err()
			{
				log::error!("🧰 Failed implementing full schedule, aborting");
				return weight;
			}
		}

		weight
	}
}
