use crate::allocation::Allocation;
use crate::Config;

use polkadot_sdk::*;

use frame_support::pallet_prelude::*;
use sp_runtime::traits::Hash;

use time_primitives::{AccountId, Balance};

use crate::airdrops::{
	AirdropBalanceOf, AirdropMintStage, AirdropTransferStage, RawAirdropMintStage,
	RawAirdropTransferStage,
};
use crate::deposits::{BalanceOf, DepositStage, RawDepositStage, RawVestedDepositStage};

/// Enum describing the migration type and data to use for an individual
/// launch stage. Uses static and 'raw' typing to allow for the use of
/// easy auditable code-embedded data formats.
#[derive(PartialEq)]
pub enum Stage {
	Retired,
	DepositFromUnlocked(RawVestedDepositStage),
	DepositAsVested(RawDepositStage),
	AirdropFromUnlocked(RawAirdropMintStage),
	AirdropTransfer(RawAirdropTransferStage),
}

/// Coupling of the different supported migrations to the raw data
/// attached to the enum data of each migration step.
impl Stage {
	/// Only retired stages lack the required data that allows us
	/// to run the migration on the current state.
	pub fn is_executable(&self) -> bool {
		*self != Stage::Retired
	}

	/// Check that all migrations can be parsed, do not run
	/// outside of tests, as it this emits events on error;
	#[cfg(test)]
	pub fn check<T: Config>(&self)
	where
		T::AccountId: From<AccountId>,
		Balance: From<BalanceOf<T>>,
	{
		use Stage::*;
		match self {
			Retired => (),
			DepositFromUnlocked(raw) => {
				DepositStage::<T>::parse_with_schedule(raw);
			},
			DepositAsVested(raw) => {
				DepositStage::<T>::parse(raw);
			},
			AirdropFromUnlocked(raw) => {
				AirdropMintStage::<T>::parse(raw);
			},
			AirdropTransfer(raw) => {
				AirdropTransferStage::<T>::parse(raw);
			},
		};
	}

	/// Provide a hash of the input data, so source data can be easier verified.
	pub fn hash<T: Config>(&self) -> T::Hash {
		use Stage::*;
		match self {
			Retired => T::Hash::default(),
			DepositFromUnlocked(raw) => T::Hashing::hash_of(raw),
			DepositAsVested(raw) => T::Hashing::hash_of(raw),
			AirdropFromUnlocked(raw) => T::Hashing::hash_of(raw),
			AirdropTransfer(raw) => T::Hashing::hash_of(raw),
		}
	}

	/// Provide amount of issuance create by migration
	pub fn sum<T: Config>(&self) -> Balance
	where
		T::AccountId: From<AccountId>,
		Balance: From<BalanceOf<T>> + From<AirdropBalanceOf<T>>,
	{
		use Stage::*;
		match self {
			Retired => 0,
			DepositFromUnlocked(raw) => DepositStage::<T>::parse_with_schedule(raw).total().into(),
			DepositAsVested(raw) => DepositStage::<T>::parse(raw).total().into(),
			AirdropFromUnlocked(raw) => AirdropMintStage::<T>::parse(raw).total().into(),
			AirdropTransfer(raw) => AirdropTransferStage::<T>::parse(raw).total().into(),
		}
	}

	/// Execute the encoded migration type on the included data.
	pub fn execute<T: Config>(&self, source: Allocation) -> Weight
	where
		T::AccountId: From<AccountId>,
		Balance: From<BalanceOf<T>>,
	{
		use Stage::*;
		match self {
			Retired => Weight::zero(),
			DepositFromUnlocked(raw) => {
				DepositStage::<T>::parse_with_schedule(raw).transfer_unlocked(source)
			},
			DepositAsVested(raw) => DepositStage::<T>::parse(raw).transfer_as_vested(source),
			AirdropFromUnlocked(raw) => AirdropMintStage::<T>::parse(raw).mint(source),
			AirdropTransfer(raw) => AirdropTransferStage::<T>::parse(raw).transfer(),
		}
	}
}
