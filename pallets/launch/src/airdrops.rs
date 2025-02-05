use crate::{Config, Event, Pallet, RawVestingSchedule};

use polkadot_sdk::*;

use frame_support::pallet_prelude::*;
use frame_support::traits::{Currency, ExistenceRequirement, VestingSchedule, WithdrawReasons};
use frame_system::pallet_prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::{AccountIdConversion, CheckedConversion, Zero};

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
pub type RawAirdropMint = (&'static str, Balance, Option<RawVestingSchedule>);

/// List of airdrops claims embedded in code, but not yet parsed and verified
pub type RawAirdropMintStage = &'static [RawAirdropMint];

/// Parsed vesting details
type AirdropVestingDetails<T> = (AirdropBalanceOf<T>, AirdropBalanceOf<T>, BlockNumberFor<T>);

/// Parsed deposit details
type AirdropMint<T> = (AccountId, AirdropBalanceOf<T>, Option<AirdropVestingDetails<T>>);

/// Parsed and verified migration that endows account
pub struct AirdropMintStage<T: Config>(Vec<AirdropMint<T>>);

impl<T: Config> AirdropMintStage<T>
where
	T::AccountId: From<AccountId>,
{
	/// Create new endowing migration by parsing and converting raw info
	pub fn parse(data: RawAirdropMintStage) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_mint(details) {
				checked.push(parsed)
			} else {
				Pallet::<T>::deposit_event(Event::<T>::AirdropMintInvalid { id: details.0.into() });
			}
		}
		Self(checked)
	}

	/// Try to parse an individual entry of an airdrop migration
	fn parse_mint(details: &RawAirdropMint) -> Option<AirdropMint<T>> {
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
			// Can fail if claim already exists, so those are logged.
			match pallet_airdrop::Pallet::<T>::mint_airdrop(target.clone(), *amount, *schedule) {
				Ok(_) => (),
				// Give detailed feedback on common failures
				Err(AirdropError::<T>::AlreadyHasClaim) => {
					Pallet::<T>::deposit_event(Event::<T>::AirdropMintExists {
						target: target.clone().into(),
					})
				},
				// Amount is below allowed minimum, should never happen
				Err(_) => Pallet::<T>::deposit_event(Event::<T>::AirdropMintFailed),
			}

			// (Read and write claims and total and optional write vesting.)
			weight += T::DbWeight::get().reads_writes(2, 3);
		}

		weight
	}

	/// Add all airdrops inside the airdrop migration
	pub fn withdraw(self, source: &[u8]) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, schedule) in self.0.iter() {
			// Burn tokens from the specified virtual wallet
			let account: T::AccountId = T::PalletId::get().into_sub_account_truncating(source);
			match AirdropCurrencyOf::<T>::withdraw(
				&account,
				*amount,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::AllowDeath,
			) {
				Ok(_) => (),
				Err(_) => {
					// Virtual wallet has run out of funds
					Pallet::<T>::deposit_event(Event::<T>::AirdropMintFailed);
					continue;
				},
			}

			// Can fail if claim already exists, so those are logged.
			match pallet_airdrop::Pallet::<T>::mint_airdrop(target.clone(), *amount, *schedule) {
				Ok(_) => (),
				// Give detailed feedback on common failures
				Err(AirdropError::<T>::AlreadyHasClaim) => {
					Pallet::<T>::deposit_event(Event::<T>::AirdropMintExists {
						target: target.clone().into(),
					})
				},
				// Amount is below allowed minimum, should never happen
				Err(_) => Pallet::<T>::deposit_event(Event::<T>::AirdropMintFailed),
			}

			// (Read and write claims and total and optional write vesting.)
			weight += T::DbWeight::get().reads_writes(2, 3);
		}

		weight
	}

	/// Add all airdrops inside the airdrop migration
	pub fn mint_or_add(self) -> Weight {
		let mut weight = Weight::zero();

		for (target, amount, schedule) in self.0.iter() {
			// Can fail if claim already exists, so those are logged.
			match pallet_airdrop::Pallet::<T>::add_airdrop(target.clone(), *amount, *schedule) {
				Ok(_) => (),
				// Give detailed feedback on common failures
				Err(AirdropError::<T>::VestingNotPossible) => {
					Pallet::<T>::deposit_event(Event::<T>::AirdropMintExists {
						target: target.clone().into(),
					})
				},
				// Amount is below allowed minimum, should never happen
				Err(_) => Pallet::<T>::deposit_event(Event::<T>::AirdropMintFailed),
			}

			// (Read and write claims and total, read and optional write vesting.)
			weight += T::DbWeight::get().reads_writes(3, 3);
		}

		weight
	}
}

type AirdropError<T> = pallet_airdrop::Error<T>;

/// Individual airdrop claim with optional vesting, embedded in code, but not yet parsed and verified
pub type RawAirdropTransfer = (&'static str, &'static str);

/// List of airdrops claims embedded in code, but not yet parsed and verified
pub type RawAirdropTransferStage = &'static [RawAirdropTransfer];

/// Parsed and verified migration that endows account
pub struct AirdropTransferStage<T: Config>(Vec<(AccountId, AccountId)>, PhantomData<T>);

impl<T: Config> AirdropTransferStage<T>
where
	T::AccountId: From<AccountId>,
{
	/// Create new endowing migration by parsing and converting raw info
	pub fn parse(data: RawAirdropTransferStage) -> Self {
		let mut checked = vec![];
		for details in data.iter() {
			if let Some(parsed) = Self::parse_transfer(details) {
				checked.push(parsed)
			} else {
				Pallet::<T>::deposit_event(Event::<T>::AirdropTransferInvalid {
					id: details.0.into(),
				});
			}
		}
		Self(checked, Default::default())
	}

	/// Try to parse an individual entry of an airdrop migration
	fn parse_transfer(details: &RawAirdropTransfer) -> Option<(AccountId, AccountId)> {
		Some((
			AccountId::from_ss58check(details.0).ok()?,
			AccountId::from_ss58check(details.1).ok()?,
		))
	}

	/// Compute the total amount of minted tokens in this migration
	pub fn total(&self) -> AirdropBalanceOf<T> {
		Zero::zero()
	}

	/// Add all airdrops inside the airdrop migration
	pub fn transfer(self) -> Weight {
		let mut weight = Weight::zero();

		for (from, to) in self.0.iter() {
			// Can fail if 'to' already exists or 'from' is missing, so those are logged.
			match pallet_airdrop::Pallet::<T>::move_airdrop(from.clone(), to.clone()) {
				Ok(_) => (),
				// Give detailed feedback on common failures
				Err(AirdropError::<T>::HasNoClaim) => {
					Pallet::<T>::deposit_event(Event::<T>::AirdropTransferMissing {
						from: from.clone().into(),
					})
				},
				Err(AirdropError::<T>::AlreadyHasClaim) => {
					Pallet::<T>::deposit_event(Event::<T>::AirdropTransferExists {
						to: to.clone().into(),
					})
				},
				// Unknown error, should never happen
				Err(_) => Pallet::<T>::deposit_event(Event::<T>::AirdropTransferFailed),
			}

			// (Read and write claims at 'from' and 'to' and optional vesting too.)
			weight += T::DbWeight::get().reads_writes(3, 4);
		}

		weight
	}
}
