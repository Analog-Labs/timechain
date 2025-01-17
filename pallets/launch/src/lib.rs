//! # Mainnet Rollout Migrations
//!
//! This pallet is responsible to migrate the mainnet chain state
//! through the various phases of the mainnet soft-launch, rollout and
//! token genesis event.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use time_primitives::{AccountId, Balance, BlockNumber};

use polkadot_sdk::*;

use sp_runtime::traits::Zero;

/// Underlying migration data
mod data;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::traits::{
		Currency, OriginTrait, PalletInfoAccess, StorageVersion, VestingSchedule,
	};
	use sp_runtime::traits::CheckedConversion;
	use sp_std::{vec, vec::Vec};

	use time_primitives::traits::Ss58Codec;

	/// Updating this number will automatically execute the next launch stages on update
	const LAUNCH_STAGE: StorageVersion = StorageVersion::new(0);

	/// Workaround to get raw storage version
	pub fn on_chain_launch_stage<P: PalletInfoAccess>() -> u16 {
		frame_support::storage::unhashed::get_or_default(&StorageVersion::storage_key::<P>()[..])
	}

	#[pallet::pallet]
	#[pallet::storage_version(LAUNCH_STAGE)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_airdrop::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// The vesting backend to use to enforce provided vesting schedules
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An new migration stage was entered
		MigrationStarted { stage: u16 },
		/// Missing migration to reach latest stage.
		MigrationMissing { stage: u16 },
		/// A deposit migration could not be parsed
		DepositInvalid { id: Vec<u8> },
		/// A deposit migration failed due to a vesting schedule conflict.
		DepositFailed { target: T::AccountId },
		/// An airdrop migration could not be parsed
		AirdropInvalid { id: Vec<u8> },
		/// An airdrop migration failed due to a vesting schedule conflict.
		AirdropFailed { target: T::AccountId },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as polkadot_sdk::frame_system::Config>::AccountId: From<AccountId>,
	{
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let mut weight = Weight::zero();

			while Pallet::<T>::on_chain_storage_version() < LAUNCH_STAGE {
				let stage = on_chain_launch_stage::<Pallet<T>>();

				Pallet::<T>::deposit_event(Event::<T>::MigrationStarted { stage });

				weight += match stage {
					0 => DepositMigration::<T>::new(data::v1::DEPOSITS_PRELAUNCH).execute(),
					1 => AirdropMigration::<T>::new(data::v2::AIRDROP_SNAPSHOT_ONE).execute(),
					_ => break,
				};

				StorageVersion::new(stage + 1).put::<Pallet<T>>();
			}

			if Pallet::<T>::on_chain_storage_version() < LAUNCH_STAGE {
				Self::deposit_event(Event::<T>::MigrationMissing {
					stage: on_chain_launch_stage::<Pallet<T>>(),
				});
			}

			weight
		}
	}

	/// Vesting schedule embedded in code, but not yet parsed and verified
	pub type RawVestingSchedule = (Balance, Balance, BlockNumber);

	/// Endowment detailsembedded in code, but not yet parsed and verified
	pub type RawEndowmentDetails = (&'static str, Balance, Option<RawVestingSchedule>);

	/// Endowment migration embedded in code, but not yet parsed and verified
	pub type RawEndowmentMigration = &'static [RawEndowmentDetails];

	/// Type aliases for currency used by the vesting schedule
	type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<
		<T as frame_system::Config>::AccountId,
	>>::Currency;

	/// Type aliases for balance used by the vesting schedule
	type BalanceOf<T> =
		<CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Parsed vesting details
	type VestingDetails<T> = (BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>);

	/// Parsed deposit details
	type DepositDetails<A, T> = (A, BalanceOf<T>, Option<VestingDetails<T>>);

	/// Parsed and verified migration that endows account
	pub struct DepositMigration<T: Config>(Vec<DepositDetails<T::AccountId, T>>);

	impl<T: Config> DepositMigration<T>
	where
		T::AccountId: From<AccountId>,
	{
		/// Create new deposit migration by parsing and converting raw info
		pub fn new(data: RawEndowmentMigration) -> Self {
			let mut checked = vec![];
			for details in data.into_iter() {
				if let Some(parsed) = Self::parse(details) {
					checked.push(parsed)
				} else {
					Pallet::<T>::deposit_event(Event::<T>::DepositInvalid { id: details.0.into() });
				}
			}
			Self(checked)
		}

		/// Parse an individual entry of a deposit migration
		pub fn parse(details: &RawEndowmentDetails) -> Option<DepositDetails<T::AccountId, T>> {
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

		/// Compute the total amount of minted token in this
		pub fn sum(self) -> BalanceOf<T> {
			self.0.iter().fold(Zero::zero(), |acc: BalanceOf<T>, &(_, b, _)| acc + b)
		}

		/// Execute deposits as far as possible, log failed deposit as events
		pub fn execute(self) -> Weight {
			let mut weight = Weight::zero();

			for (target, amount, schedule) in self.0.iter() {
				// Check vesting status first...
				if schedule.is_some()
					&& <T as Config>::VestingSchedule::vesting_balance(&target).is_some()
				{
					Pallet::<T>::deposit_event(Event::<T>::DepositFailed {
						target: target.clone(),
					});
					continue;
				}

				// ...then add balance to ensure that the account exists...
				let _ = CurrencyOf::<T>::deposit_creating(&target, *amount);

				// ...and finally apply vesting schedule, if there is one.
				if let Some(vs) = schedule {
					<T as Config>::VestingSchedule::add_vesting_schedule(&target, vs.0, vs.1, vs.2)
						.expect("No other vesting schedule exists, as checked above; qed");
				}

				//let count = 10;
				//weight += T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
			}

			weight
		}
	}

	/// Type aliases for the currency used in the airdrop vesting scheduler
	type AirdropCurrencyOf<T> =
		<<T as pallet_airdrop::Config>::VestingSchedule as VestingSchedule<
			<T as frame_system::Config>::AccountId,
		>>::Currency;

	/// Type aliases for the balance used in the airdrop vesting scheduler
	type AirdropBalanceOf<T> =
		<AirdropCurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Parsed vesting details
	type AirdropVestingDetails<T> = (AirdropBalanceOf<T>, AirdropBalanceOf<T>, BlockNumberFor<T>);

	/// Parsed deposit details
	type AirdropDetails<T> = (AccountId, AirdropBalanceOf<T>, Option<AirdropVestingDetails<T>>);

	/// Parsed and verified migration that endows account
	pub struct AirdropMigration<T: Config>(Vec<AirdropDetails<T>>);

	impl<T: Config> AirdropMigration<T>
	where
		T::AccountId: From<AccountId>,
	{
		/// Create new endowing migration by parsing and converting raw info
		pub fn new(data: RawEndowmentMigration) -> Self {
			let mut checked = vec![];
			for details in data.into_iter() {
				if let Some(parsed) = Self::parse(details) {
					checked.push(parsed)
				} else {
					Pallet::<T>::deposit_event(Event::<T>::AirdropInvalid { id: details.0.into() });
				}
			}
			Self(checked)
		}

		/// Try to parse an individual entry of an airdrop migration
		pub fn parse(details: &RawEndowmentDetails) -> Option<AirdropDetails<T>> {
			Some((
				AccountId::from_ss58check(details.0).ok()?.into(),
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

		/// Add all airdrops inside the airdrop migration
		pub fn execute(self) -> Weight {
			let mut weight = Weight::zero();

			for (target, amount, schedule) in self.0.iter() {
				if pallet_airdrop::Pallet::<T>::mint(
					T::RuntimeOrigin::root(),
					target.clone(),
					*amount,
					*schedule,
				)
				.is_err()
				{
					Pallet::<T>::deposit_event(Event::<T>::AirdropFailed {
						target: target.clone().into(),
					});
					continue;
				}

				//let count = 10;
				//weight += T::DbWeight::get().reads_writes(count as Weight + 1, count as Weight + 1)
			}

			weight
		}
	}
}
