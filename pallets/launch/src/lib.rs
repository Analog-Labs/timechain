#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
//! # Mainnet Rollout Migrations
//!
//! This pallet is responsible to migrate the mainnet chain state
//! through the various phases of the mainnet soft-launch, rollout and
//! token genesis event.

pub use pallet::*;

use time_primitives::{AccountId, Balance, BlockNumber, ANLOG, MILLIANLOG};

use polkadot_sdk::*;

/// Underlying migration data
mod data;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod benchmarks;

mod airdrops;
mod allocation;
mod application;
mod deposits;
mod ledger;
mod stage;

use airdrops::AirdropBalanceOf;
use allocation::Allocation;
use application::Application;
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
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{
		Currency, ExistenceRequirement, LockableCurrency, StorageVersion, WithdrawReasons,
	};
	use frame_support::PalletId;
	use frame_system::pallet_prelude::*;
	use sp_runtime::TokenError;
	use sp_std::{vec, vec::Vec};

	pub trait WeightInfo {
		fn lock_operational() -> Weight;
	}

	pub struct TestWeightInfo;
	impl WeightInfo for TestWeightInfo {
		fn lock_operational() -> Weight {
			Weight::zero()
		}
	}

	/// Updating this number will automatically execute the next launch stages on update
	pub const LAUNCH_VERSION: u16 = 30;
	/// Wrapped version to support substrate interface as well
	pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(LAUNCH_VERSION);

	/// The official mainnet launch ledger
	pub const LAUNCH_LEDGER: RawLaunchLedger = &[
		// Genesis
		(0, Allocation::Ecosystem, 3100 * ANLOG, Stage::Retired),
		// Prelaunch Deposit 1
		(1, Allocation::Ecosystem, 53_030_500 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 1
		(2, Allocation::Airdrop, 410_168_624 * ANLOG, Stage::Retired),
		(3, Allocation::Airdrop, 0, Stage::Retired),
		(4, Allocation::Airdrop, 0, Stage::Retired),
		// Prelaunch Deposit 2
		(5, Allocation::Ecosystem, 39_328_063 * ANLOG, Stage::Retired),
		// Airdrop Testing
		(6, Allocation::Ecosystem, 50 * ANLOG, Stage::Retired),
		// Prelaunch Deposit 3
		(7, Allocation::Ecosystem, 226_449_338 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 2
		(8, Allocation::Airdrop, 20_288_873 * ANLOG, Stage::Retired),
		// Validator Airdrop
		(9, Allocation::Ecosystem, 21_111_617 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 3
		(10, Allocation::Airdrop, 1_373_348 * ANLOG, Stage::Retired),
		// Airdrop address updates
		(11, Allocation::Airdrop, 0, Stage::Retired),
		// Airdrop Snapshot 1 - Missing EVM wallet
		(12, Allocation::Airdrop, 105_317 * ANLOG, Stage::Retired),
		// Prelaunch Deposit 4
		(13, Allocation::Ecosystem, 113_204_200 * ANLOG, Stage::Retired),
		// Virtual Token Genesis Event
		(14, Allocation::Ignore, 8_166_845_674 * ANLOG, Stage::Retired),
		// Retry failed mints in stage 9
		(15, Allocation::Ecosystem, 6_062_296 * ANLOG, Stage::Retired),
		// NOTE: Minting stopped here, all funds are either in virtual wallets or unclaimed airdrops
		// Airdrop Snapshot 4
		(16, Allocation::Airdrop, 1_336_147_462_613_682_971, Stage::Retired),
		// Airdrop Move 2 (+ accounting for upcoming Staking Allocation)
		(17, Allocation::Initiatives, 1_100_010 * ANLOG, Stage::Retired),
		// Prelaunch Deposit 4
		(18, Allocation::Ecosystem, 3_636_364 * ANLOG, Stage::Retired),
		// Bootstaking Month 1
		(19, Allocation::Initiatives, 60_386_473 * ANLOG, Stage::Retired),
		// Provide fjord sale tokens to claims backend
		(20, Allocation::Ecosystem, 116_163_163 * ANLOG, Stage::Retired),
		// Testing vested transfers
		(21, Allocation::Team, 45_289_855 * ANLOG, Stage::Retired),
		// Launchday transfers 1
		(22, Allocation::Ecosystem, 49_081_886_536 * MILLIANLOG, Stage::Retired),
		// OTC Preparation 1
		(23, Allocation::Ecosystem, 22_644_927_500 * MILLIANLOG, Stage::Retired),
		// OTC Preparation 2
		(24, Allocation::Initiatives, 249_094_202_500 * MILLIANLOG, Stage::Retired),
		// OTC Preparation 3
		(25, Allocation::Initiatives, 362_318_840 * ANLOG, Stage::Retired),
		// Launchday transfer 2
		(26, Allocation::Ecosystem, 14_449_903_350 * MILLIANLOG, Stage::Retired),
		// Bridged token allocation
		(27, Allocation::Initiatives, 45_289_855 * ANLOG, Stage::Retired),
		// Airdrop Snapshot 5
		(28, Allocation::Airdrop, 1_097_142_834_936_105_265, Stage::Retired),
		// Airdrop Move 3
		(29, Allocation::Airdrop, 0, Stage::Retired),
		// Validator Airdrop (missed)
		(30, Allocation::Ecosystem, 160_086 * ANLOG, Stage::Retired),
	];

	/// TODO: Difference that was actually minted for airdrops:
	/// stage_2 = 410_168_624 * ANLOG - 410_168_623_085_944_989_935
	/// stage_8 = 20_288_873 * ANLOG - 20_288_872_847_294_611_363
	/// stage_10 = 1_373_348 * ANLOG - 1_373_347_559_383_359_315
	/// stage_12 = 105_317 * ANLOG - 105_316_962_722_110_899
	/// stage_16 = 1_336_148 * ANLOG - 1_336_147_462_613_682_971
	/// stage_28 = 1_097_144 * ANLOG - 1_097_142_834_936_105_265

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		polkadot_sdk::frame_system::Config + pallet_vesting::Config + pallet_airdrop::Config
	{
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		/// Identifier to use for pallet-owned wallets
		type PalletId: Get<PalletId>;
		/// The minimum size a deposit has to have to be considered valid.
		/// Set this to at least the minimum deposit to avoid errors
		type MinimumDeposit: Get<BalanceOf<Self>>;
		/// Allowed origin for authorized calls
		type LaunchAdmin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Weight information of the pallet
		type WeightInfo: WeightInfo;
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
		/// A deposit migration could not be executed due to a source missmatching.
		DepositSourceMissmatch { source: Vec<u8> },
		/// A vested deposit failed due to a missmatch
		DepositVestingMissmatch { target: T::AccountId },
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

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Update total amount of tokens that are locked in one of the operational wallets.
		///
		/// This is used as a preparation for miniting, as a result of burning wrapped
		/// tokens on another chain or other tokenomics reasons.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::lock_operational())]
		pub fn lock_operational(
			origin: OriginFor<T>,
			target: Application,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::LaunchAdmin::ensure_origin(origin)?;

			let account = target.account_id::<T>();
			ensure!(
				CurrencyOf::<T>::total_balance(&account) >= amount,
				TokenError::FundsUnavailable
			);
			CurrencyOf::<T>::set_lock(target.lock_id(), &account, amount, WithdrawReasons::all());

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		Balance: From<BalanceOf<T>> + From<AirdropBalanceOf<T>>,
	{
		/// Compute the account ID of any virtual source wallet.
		pub fn account_id(source: Allocation) -> T::AccountId {
			source.account_id::<T>()
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
			source: Allocation,
			target: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			CurrencyOf::<T>::transfer(
				&source.account_id::<T>(),
				&target,
				amount,
				ExistenceRequirement::AllowDeath,
			)?;
			Pallet::<T>::deposit_event(Event::<T>::TransferFromVirtual {
				source: source.sub_id().to_vec(),
				target,
				amount,
			});
			Ok(())
		}
	}
}
