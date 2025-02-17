use crate::allocation::Allocation;
use crate::{Config, Event, Pallet, RawVestingSchedule};

use polkadot_sdk::*;

use frame_support::pallet_prelude::*;
use frame_support::traits::{Currency, ExistenceRequirement, VestingSchedule};
use frame_system::pallet_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::{CheckedConversion, Zero};
use sp_std::{vec, vec::Vec};

use time_primitives::{AccountId, Balance};

/// Type aliases for currency used by the vesting schedule
pub type CurrencyOf<T> = <T as pallet_vesting::Config>::Currency;

/// Type aliases for balance used by the vesting schedule
pub type BalanceOf<T> =
	<CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Individual balance transfer embedded in code, but not yet parsed and verified
pub type RawDeposit = (&'static str, Balance);

/// List of deposit migrations embedded in code, but not yet parsed and verified
pub type RawDepositStage = &'static [RawDeposit];

/// Individual deposit with optional vesting, embedded in code, but not yet parsed and verified
pub type RawDepositWithSchedule = (&'static str, Balance, Option<RawVestingSchedule>);

/// List of deposit migrations embedded in code, but not yet parsed and verified
pub type RawVestedDepositStage = &'static [RawDepositWithSchedule];

/// Parsed vesting details type alias
pub type VestingDetails<T> = (BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>);

/// Parsed deposit details type alias
type DepositDetails<A, T> = (A, BalanceOf<T>, Option<VestingDetails<T>>);

/// Parsed and verified migration that endows account
pub struct DepositStage<T: Config>(Vec<DepositDetails<T::AccountId, T>>);

impl<T: Config> DepositStage<T>
where
	T::AccountId: From<AccountId>,
	Balance: From<BalanceOf<T>>,
{
	/// Create new deposit migration by parsing and converting raw info
	pub fn parse(data: RawDepositStage) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_details(details) {
				if parsed.1 < <T as Config>::MinimumDeposit::get() {
					Pallet::<T>::deposit_event(Event::<T>::DepositTooSmall { target: parsed.0 });
					continue;
				}

				checked.push(parsed)
			} else {
				Pallet::<T>::deposit_event(Event::<T>::DepositInvalid { id: details.0.into() });
			}
		}
		Self(checked)
	}

	/// Parse an individual entry of a deposit migration
	fn parse_details(details: &RawDeposit) -> Option<DepositDetails<T::AccountId, T>> {
		Some((
			AccountId::from_ss58check(details.0).ok()?.into(),
			BalanceOf::<T>::checked_from(details.1)?,
			None,
		))
	}

	/// Create new deposit migration by parsing and converting raw info
	pub fn parse_with_schedule(data: RawVestedDepositStage) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_scheduled_details(details) {
				if parsed.1 < <T as Config>::MinimumDeposit::get() {
					Pallet::<T>::deposit_event(Event::<T>::DepositTooSmall { target: parsed.0 });
					continue;
				}

				checked.push(parsed)
			} else {
				Pallet::<T>::deposit_event(Event::<T>::DepositInvalid { id: details.0.into() });
			}
		}
		Self(checked)
	}

	/// Parse an individual entry of a deposit migration
	fn parse_scheduled_details(
		details: &RawDepositWithSchedule,
	) -> Option<DepositDetails<T::AccountId, T>> {
		Some((
			AccountId::from_ss58check(details.0).ok()?.into(),
			BalanceOf::<T>::checked_from(details.1)?,
			if let Some((locked, per_block, start)) = details.2 {
				Some((
					BalanceOf::<T>::checked_from(locked)?,
					BalanceOf::<T>::checked_from(per_block)?,
					BlockNumberFor::<T>::checked_from(start)?,
				))
			} else {
				None
			},
		))
	}

	/// Compute the total amount of minted tokens in this migration
	pub fn total(&self) -> BalanceOf<T> {
		self.0.iter().fold(Zero::zero(), |acc: BalanceOf<T>, &(_, b, _)| acc + b)
	}

	/// Execute deposits as far as possible, log failed deposit as events
	pub fn transfer_unlocked(self, source: Allocation) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, schedule) in self.0.iter() {
			// Check vesting status first...
			if let Some(vs) = schedule {
				// (Checking if the target is able to receive a vested transfer is one read)
				weight += T::DbWeight::get().reads(1);

				if pallet_vesting::Pallet::<T>::can_add_vesting_schedule(target, vs.0, vs.1, vs.2)
					.is_err()
				{
					Pallet::<T>::deposit_event(Event::<T>::DepositFailed {
						target: target.clone(),
					});
					continue;
				}
			}

			// (Read and write source, read existential deposit, followed by reading and writing target.)
			weight += T::DbWeight::get().reads_writes(3, 2);

			// ... then attempt to transfer tokens directly from virtual deposit
			if CurrencyOf::<T>::transfer(
				&source.account_id::<T>(),
				target,
				*amount,
				ExistenceRequirement::AllowDeath,
			)
			.is_err()
			{
				Pallet::<T>::deposit_event(Event::<T>::DepositTooLarge { target: target.clone() });
				continue;
			}

			// ...and finally apply vesting schedule, if there is one.
			if let Some(vs) = schedule {
				// (Updating the vesting schedule involves reading the info, followed by writing the info, the vesting and the lock.)
				weight += T::DbWeight::get().reads_writes(1, 3);

				pallet_vesting::Pallet::<T>::add_vesting_schedule(target, vs.0, vs.1, vs.2)
					.expect("No other vesting schedule exists, as checked above; qed");
			}
		}
		weight
	}

	/// Execute deposits as far as possible, log failed deposit as events
	pub fn transfer_as_vested(self, source: Allocation) -> Weight {
		let mut weight = Weight::zero();

		let account = source.account_id::<T>();

		// Ensure account is fully vested
		if pallet_vesting::Pallet::<T>::vesting_balance(&account)
			!= Some(CurrencyOf::<T>::free_balance(&account))
		{
			Pallet::<T>::deposit_event(Event::<T>::DepositSourceMissmatch {
				source: source.sub_id().to_vec(),
			});
			return weight;
		}

		// Remove existing schedule during operation
		if pallet_vesting::Pallet::<T>::remove_vesting_schedule(&account, 0).is_err() {
			Pallet::<T>::deposit_event(Event::<T>::DepositSourceMissmatch {
				source: source.sub_id().to_vec(),
			});
			return weight;
		}

		// Handle all the transfers...
		for (target, amount, no_schedule) in self.0.iter() {
			// (Read and write source and schedule, read existential deposit, followed by reading and writing target.)
			weight += T::DbWeight::get().reads_writes(5, 5);

			// Allocation needs to be the sole provided of the vesting schedule
			if no_schedule.is_some() {
				Pallet::<T>::deposit_event(Event::<T>::DepositVestingMissmatch {
					target: target.clone(),
				});
				continue;
			}

			// Compute relative vesting schedule
			if let Some(vs) = source.schedule_rel::<T>(*amount) {
				if pallet_vesting::Pallet::<T>::can_add_vesting_schedule(target, vs.0, vs.1, vs.2)
					.is_err()
				{
					Pallet::<T>::deposit_event(Event::<T>::DepositFailed {
						target: target.clone(),
					});
					continue;
				}

				// ... then attempt to transfer tokens directly from virtual deposit
				if CurrencyOf::<T>::transfer(
					&account,
					target,
					*amount,
					ExistenceRequirement::AllowDeath,
				)
				.is_err()
				{
					Pallet::<T>::deposit_event(Event::<T>::DepositTooLarge {
						target: target.clone(),
					});
					continue;
				}

				pallet_vesting::Pallet::<T>::add_vesting_schedule(target, vs.0, vs.1, vs.2)
					.expect("No other vesting schedule exists, as checked above; qed");
			} else {
				Pallet::<T>::deposit_event(Event::<T>::DepositVestingMissmatch {
					target: target.clone(),
				});
				continue;
			}
		}

		// Ensure the remaining tokens are locked again
		let remaining = CurrencyOf::<T>::free_balance(&account);
		if let Some(vs) = source.schedule_rel::<T>(remaining) {
			pallet_vesting::Pallet::<T>::add_vesting_schedule(&account, vs.0, vs.1, vs.2)
				.expect("Previous vesting schedule war removed; qed");
		} else {
			Pallet::<T>::deposit_event(Event::<T>::DepositSourceMissmatch {
				source: source.sub_id().to_vec(),
			});
		}

		weight
	}
}
