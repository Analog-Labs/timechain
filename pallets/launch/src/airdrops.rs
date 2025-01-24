use crate::{Config, Event, Pallet, RawVestingSchedule};

use polkadot_sdk::*;

use frame_support::pallet_prelude::*;
use frame_support::traits::{Currency, OriginTrait, VestingSchedule};
use frame_system::pallet_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::{CheckedConversion, Zero};

use sp_std::{vec, vec::Vec};

use time_primitives::{AccountId, Balance};

/// Type aliases for the currency used in the airdrop vesting scheduler
type AirdropCurrencyOf<T> = <<T as pallet_airdrop::Config>::VestingSchedule as VestingSchedule<
	<T as frame_system::Config>::AccountId,
>>::Currency;

/// Type aliases for the balance used in the airdrop vesting scheduler
pub type AirdropBalanceOf<T> =
	<AirdropCurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Individual airdrop claim with optional vesting, embedded in code, but not yet parsed and verified
pub type RawAirdropDetails = (&'static str, Balance, Option<RawVestingSchedule>);

/// List of airdrops claims embedded in code, but not yet parsed and verified
pub type RawAirdropStage = &'static [RawAirdropDetails];

/// Parsed vesting details
type AirdropVestingDetails<T> = (AirdropBalanceOf<T>, AirdropBalanceOf<T>, BlockNumberFor<T>);

/// Parsed deposit details
type AirdropDetails<T> = (AccountId, AirdropBalanceOf<T>, Option<AirdropVestingDetails<T>>);

/// Parsed and verified migration that endows account
pub struct AirdropStage<T: Config>(Vec<AirdropDetails<T>>);

impl<T: Config> AirdropStage<T>
where
	T::AccountId: From<AccountId>,
{
	/// Create new endowing migration by parsing and converting raw info
	pub fn parse(data: RawAirdropStage) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_details(details) {
				checked.push(parsed)
			} else {
				Pallet::<T>::deposit_event(Event::<T>::AirdropInvalid { id: details.0.into() });
			}
		}
		Self(checked)
	}

	/// Try to parse an individual entry of an airdrop migration
	fn parse_details(details: &RawAirdropDetails) -> Option<AirdropDetails<T>> {
		Some((
			AccountId::from_ss58check(details.0).ok()?,
			AirdropBalanceOf::<T>::checked_from(details.1)?,
			if let Some((locked, per_block, start)) = details.2 {
				Some((
					AirdropBalanceOf::<T>::checked_from(locked)?,
					AirdropBalanceOf::<T>::checked_from(per_block)?,
					BlockNumberFor::<T>::checked_from(start)?,
				))
			} else {
				None
			},
		))
	}

	/// Compute the total amount of minted tokens in this migration
	pub fn total(&self) -> AirdropBalanceOf<T> {
		self.0.iter().fold(Zero::zero(), |acc: AirdropBalanceOf<T>, &(_, b, _)| acc + b)
	}

	/// Add all airdrops inside the airdrop migration
	pub fn mint(self) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, schedule) in self.0.iter() {
			weight += T::DbWeight::get().reads_writes(1, 3);

			// Can fail if claim already exists, so those are logged.
			if pallet_airdrop::Pallet::<T>::mint(
				T::RuntimeOrigin::root(),
				target.clone(),
				*amount,
				*schedule,
			)
			.is_err()
			{
				// (Reading claims was necessary to determine claim exists.)
				weight += T::DbWeight::get().reads(1);

				Pallet::<T>::deposit_event(Event::<T>::AirdropFailed {
					target: target.clone().into(),
				});
				continue;
			}

			// (Read and write claims and total and optimal write vesting.)
			weight += T::DbWeight::get().reads_writes(2, 3);
		}

		weight
	}
}
