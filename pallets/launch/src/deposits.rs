use crate::{Config, Event, Pallet, RawVestingSchedule};

use polkadot_sdk::*;

use frame_support::pallet_prelude::*;
use frame_support::traits::{Currency, ExistenceRequirement, VestingSchedule};
use frame_system::pallet_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::{AccountIdConversion, CheckedConversion, Zero};
use sp_std::{vec, vec::Vec};

use time_primitives::{AccountId, Balance};

/// Type aliases for currency used by the vesting schedule
pub type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<
	<T as frame_system::Config>::AccountId,
>>::Currency;

/// Type aliases for balance used by the vesting schedule
pub type BalanceOf<T> =
	<CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Individual deposit with optional vesting, embedded in code, but not yet parsed and verified
pub type RawDepositDetails = (&'static str, Balance, Option<RawVestingSchedule>);

/// List of deposit migrations embedded in code, but not yet parsed and verified
pub type RawDepositStage = &'static [RawDepositDetails];

/// Virtual wallet identifier
pub type RawVirtualSource = &'static [u8];

/// Individual virtual deposit to pallet wallet, embedded in code, but not yet parsed and verified
pub type RawVirtualDepositDetails = (&'static [u8], Balance, Option<RawVestingSchedule>);

/// List of deposit to virtual pallet wallet, embedded in code, but not yet parsed and verified
pub type RawVirtualDepositStage = &'static [RawVirtualDepositDetails];

/// Parsed vesting details type alias
type VestingDetails<T> = (BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>);

/// Parsed deposit details type alias
type DepositDetails<A, T> = (A, BalanceOf<T>, Option<VestingDetails<T>>);

/// Parsed and verified migration that endows account
pub struct DepositStage<T: Config>(Vec<DepositDetails<T::AccountId, T>>);

impl<T: Config> DepositStage<T>
where
	T::AccountId: From<AccountId>,
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
	fn parse_details(details: &RawDepositDetails) -> Option<DepositDetails<T::AccountId, T>> {
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

	/// Create new deposit migration by parsing and converting raw info
	pub fn parse_virtual(data: RawVirtualDepositStage) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_virtual_details(details) {
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
	pub fn parse_virtual_details(
		details: &RawVirtualDepositDetails,
	) -> Option<DepositDetails<T::AccountId, T>> {
		Some((
			T::PalletId::get().into_sub_account_truncating(details.0),
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
	pub fn deposit(self) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, schedule) in self.0.iter() {
			// Check vesting status first...
			if let Some(vs) = schedule {
				// (Checking if the target is able to receive a vested transfer is one read)
				weight += T::DbWeight::get().reads(1);

				if <T as Config>::VestingSchedule::can_add_vesting_schedule(
					target, vs.0, vs.1, vs.2,
				)
				.is_err()
				{
					Pallet::<T>::deposit_event(Event::<T>::DepositFailed {
						target: target.clone(),
					});
					continue;
				}
			}

			// ...then add balance to ensure that the account exists...
			let _ = CurrencyOf::<T>::deposit_creating(target, *amount);
			// (Read existential deposit, followed by reading and writing account store.)
			weight += T::DbWeight::get().reads_writes(2, 1);

			// ...and finally apply vesting schedule, if there is one.
			if let Some(vs) = schedule {
				<T as Config>::VestingSchedule::add_vesting_schedule(target, vs.0, vs.1, vs.2)
					.expect("No other vesting schedule exists, as checked above; qed");
				// (Updating the vesting schedule involves reading the info, followed by writing the info, the vesting and the lock.)
				weight += T::DbWeight::get().reads_writes(1, 3)
			}
		}
		weight
	}

	/// Execute deposits as far as possible, log failed deposit as events
	pub fn withdraw(self, source: RawVirtualSource) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, schedule) in self.0.iter() {
			// Check vesting status first...
			if let Some(vs) = schedule {
				// (Checking if the target is able to receive a vested transfer is one read)
				weight += T::DbWeight::get().reads(1);

				if <T as Config>::VestingSchedule::can_add_vesting_schedule(
					target, vs.0, vs.1, vs.2,
				)
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
			let source: T::AccountId = T::PalletId::get().into_sub_account_truncating(source);
			if CurrencyOf::<T>::transfer(&source, target, *amount, ExistenceRequirement::AllowDeath)
				.is_err()
			{
				Pallet::<T>::deposit_event(Event::<T>::DepositFailed { target: target.clone() });
				continue;
			}

			// ...and finally apply vesting schedule, if there is one.
			if let Some(vs) = schedule {
				// (Updating the vesting schedule involves reading the info, followed by writing the info, the vesting and the lock.)
				weight += T::DbWeight::get().reads_writes(1, 3);

				<T as Config>::VestingSchedule::add_vesting_schedule(target, vs.0, vs.1, vs.2)
					.expect("No other vesting schedule exists, as checked above; qed");
			}
		}
		weight
	}
}
