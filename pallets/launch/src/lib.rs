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

/// Pallet log target constant
const LOG_TARGET: &str = "launch";

/// Vesting schedule embedded in code, but not yet parsed and verified
pub type RawVestingSchedule = (Balance, Balance, BlockNumber);

/// All pallet logic is defined in its own module and must be annotated by the `pallet` attribute.
#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	// Import various useful types required by all FRAME pallets.
	use super::*;
	use deposits::RawVirtualSource;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::traits::{Currency, ExistenceRequirement, StorageVersion, VestingSchedule};
	use frame_support::PalletId;
	use sp_runtime::traits::AccountIdConversion;
	use sp_std::{vec, vec::Vec};

	/// Updating this number will automatically execute the next launch stages on update
	pub const LAUNCH_VERSION: u16 = 20;
	/// Wrapped version to support substrate interface as well
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
		(9, 21_111_617 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 3
		(10, 1_373_347_559_383_359_315, Stage::Retired),
		// Airdrop address updates
		(11, 0, Stage::Retired),
		// Airdrop Snapshot 1 - Missing EVM wallet
		(12, 105_316_962_722_110_899, Stage::Retired),
		// Prelaunch Deposit 4
		(13, 113_204_200 * ANLOG, Stage::Retired),
		// Virtual Token Genesis Event
		(14, 8_166_845_674 * ANLOG, Stage::Retired),
		// FIXME: Minting stopped here: Toke ledger should check virtual wallets, not issuance.
		// Retry failed mints in stage 11
		(15, 6_062_296 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 4
		(16, 1_336_147_462_613_682_971, Stage::Retired),
		// Airdrop Move 2
		(17, 0, Stage::Retired),
		// Prelaunch Deposit 4
		(18, 3_636_364 * ANLOG, Stage::Retired),
		// Bootstaking Month 1
		(
			19,
			60_386_473 * ANLOG,
			Stage::DepositFromVirtual(b"initiatives", data::v19::DEPOSIT_REWARD_POOL),
		),
		(
			20,
			116_163_163 * ANLOG,
			Stage::DepositFromVirtual(b"ecosystem", data::v20::DEPOSIT_FJORD_SALE),
		),
	];

	/// TODO: Difference to go to treasury:
	/// stage_2 = 410_168_624 * ANLOG;
	/// stage_8 = 20_288_873 * ANLOG;
	/// stage_10 = 1_373_348 * ANLOG;
	/// stage_11 = 105_317 * ANLOG;
	/// stage_16 = 1_336_148 * ANLOG;

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
		/// Deposit does not exceed existential deposit.
		DepositTooSmall { target: T::AccountId },
		/// Deposit exceeds available virtual funds.
		DepositTooLarge { target: T::AccountId },
		/// A deposit migration failed due to a vesting schedule conflict.
		DepositFailed { target: T::AccountId },
		/// An airdrop mint could not be parsed.
		AirdropMintInvalid { id: Vec<u8> },
		/// An airdrop mint failed due to an existing claim.
		AirdropMintExists { target: T::AccountId },
		/// An airdrop mint exceeds available virtual funds.
		AirdropMintTooLarge { target: T::AccountId },
		/// An airdrop mint failed due to an internal error.
		AirdropMintFailed,
		/// An airdrop move could not be parsed.
		AirdropTransferInvalid { id: Vec<u8> },
		/// An airdrop move has failed due to no claim existing or having already been claimed.
		AirdropTransferMissing { from: T::AccountId },
		/// An airdrop move has failed due to an claim existing already.
		AirdropTransferExists { to: T::AccountId },
		/// An airdrop move has failed for an unknown reason.
		AirdropTransferFailed,
		/// A virtual transfer was successful.
		TransferFromVirtual { source: Vec<u8>, target: T::AccountId, amount: BalanceOf<T> },
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
				Err(error) => {
					log::error!(
						target: LOG_TARGET,
						"🪦 Failed to parse launch ledger: {:?}", error
					);

					Pallet::<T>::deposit_event(Event::<T>::LedgerInvalid { error })
				},
			}
			Weight::zero()
		}
	}

	impl<T: Config> Pallet<T>
	where
		Balance: From<BalanceOf<T>> + From<AirdropBalanceOf<T>>,
	{
		/// Compute the account ID of any virtual source wallet.
		pub fn account_id(source: RawVirtualSource) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(source)
		}

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

		/// Withdraw balance from virtual account
		pub fn transfer_virtual(
			source: RawVirtualSource,
			target: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let account = Self::account_id(source);
			CurrencyOf::<T>::transfer(&account, &target, amount, ExistenceRequirement::AllowDeath)?;
			Pallet::<T>::deposit_event(Event::<T>::TransferFromVirtual {
				source: source.to_vec(),
				target,
				amount,
			});
			Ok(())
		}
	}
}
