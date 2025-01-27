#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
//! # Mainnet Rollout Migrations
//!
//! This pallet is responsible to migrate the mainnet chain state
//! through the various phases of the mainnet soft-launch, rollout and
//! token genesis event.

pub use pallet::*;

use time_primitives::{AccountId, Balance, BlockNumber, ANLOG};

use polkadot_sdk::*;

/// Underlying migration data
mod data;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod airdrops;
mod deposits;
mod ledger;
mod stage;

use airdrops::AirdropBalanceOf;
use deposits::{BalanceOf, CurrencyOf};
use ledger::{LaunchLedger, RawLaunchLedger};
use stage::Stage;

/// Vesting schedule embedded in code, but not yet parsed and verified
pub type RawVestingSchedule = (Balance, Balance, BlockNumber);

/// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::traits::{Currency, StorageVersion, VestingSchedule};
	use frame_support::PalletId;
	use sp_std::{vec, vec::Vec};

	/// Updating this number will automatically execute the next launch stages on update
	pub const LAUNCH_VERSION: u16 = 12;
	/// Wrapped version to support sustrate interface as well
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(LAUNCH_VERSION);

	/// The official mainnet launch ledger
	pub const LAUNCH_LEDGER: RawLaunchLedger = &[
		// Genesis
		(0, 3100 * ANLOG, Stage::Retired),
		// Prelaunch Deposit 1
		(1, 53_030_500 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 1
		(2, 410_168_623_085_944_989_935, Stage::Retired),
		(3, 0, Stage::Retired),
		(4, 0, Stage::Retired),
		// Prelaunch Deposit 2
		(5, 39_328_063 * ANLOG, Stage::Retired),
		// Airdrop Testing
		(6, 50 * ANLOG, Stage::Retired),
		// Prelaunch Deposit 3
		(7, 226_449_338 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 2
		(8, 20_288_872_847_294_611_363, Stage::Retired),
		// Validator Airdrop
		(9, 27_173_913 * ANLOG, Stage::Airdrop(data::v9::AIRDROPS_VALIDATORS)),
		// Airdrop Snapshot 3
		(10, 1_373_347_559_383_359_315, Stage::Airdrop(data::v10::AIRDROPS_SNAPSHOT_3)),
		// Prelaunch Deposit 4
		(11, 113_204_200 * ANLOG, Stage::Deposit(data::v11::DEPOSITS_PRELAUNCH_4)),
		// Virtual Token Genesis Event
		(12, 8_166_950_991 * ANLOG, Stage::VirtualDeposit(data::v12::DEPOSITS_TOKEN_GENESIS_EVENT)),
	];

	/// TODO: Difference to go to treasury:
	/// stage_2 = 410_168_624 * ANLOG;
	/// stage_8 = 20_288_873 * ANLOG;
	/// stage_10 = 1_373_348 * ANLOG;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config + pallet_airdrop::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// Identifier to use for pallet-owned wallets
		type PalletId: Get<PalletId>;
		/// The vesting backend to use to enforce provided vesting schedules.
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;
		/// The minimum size a deposit has to have to be considered valid.
		/// Set this to at least the minimum deposit to avoid errors
		type MinimumDeposit: Get<BalanceOf<Self>>;
	}

	/// All error are a result of the "compile" step and do not allow execution.
	#[pallet::error]
	#[derive(Clone, PartialEq)]
	pub enum Error<T> {
		/// A required stage version is missing from the ledger.
		StageMissing,
		/// The version numbers of the stages in the ledger are inconsistent.
		StageVersionMissmatch,
		/// The current total issuance exceeds the current planned allocation.
		TotalIssuanceExceeded,
		/// Migrations needed, but retired from this runtime.
		StageRetired,
		/// Migrations needed and know, but missing in this runtime.
		StageIssuanceMissmatch,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Internal launch ledger invalid, no migration executed.
		LedgerInvalid { error: Error<T> },
		/// The migration to a new launch stage was executed, but exceeded the expected issuance.
		StageExceededIssuance { version: u16, hash: T::Hash },
		/// The migration to a new launch stage was successfully executed.
		StageExecuted { version: u16, hash: T::Hash },
		/// A deposit migration could not be parsed.
		DepositInvalid { id: Vec<u8> },
		/// Deposit does not exceed existential deposit
		DepositTooSmall { target: T::AccountId },
		/// A deposit migration failed due to a vesting schedule conflict.
		DepositFailed { target: T::AccountId },
		/// An airdrop migration could not be parsed
		AirdropInvalid { id: Vec<u8> },
		/// An airdrop migration failed due to an existing claim or a vesting
		/// schedule conflict.
		AirdropFailed { target: T::AccountId },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		T::AccountId: From<AccountId>,
		Balance: From<BalanceOf<T>> + From<AirdropBalanceOf<T>>,
	{
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			match LaunchLedger::compile(LAUNCH_LEDGER) {
				Ok(plan) => return plan.run(),
				Err(error) => Pallet::<T>::deposit_event(Event::<T>::LedgerInvalid { error }),
			}
			Weight::zero()
		}
	}

	impl<T: Config> Pallet<T>
	where
		Balance: From<BalanceOf<T>> + From<AirdropBalanceOf<T>>,
	{
		/// Workaround to get raw storage version
		pub fn on_chain_stage_version() -> u16 {
			frame_support::storage::unhashed::get_or_default(
				&StorageVersion::storage_key::<Self>()[..],
			)
		}

		/// Estimate current total issuance
		pub fn total_issuance() -> Balance {
			Into::<Balance>::into(CurrencyOf::<T>::total_issuance())
				+ Into::<Balance>::into(pallet_airdrop::Total::<T>::get())
		}
	}
}
