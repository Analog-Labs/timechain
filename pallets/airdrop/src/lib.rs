#![cfg_attr(not(feature = "std"), no_std)]
// Copyright (C) Parity Technologies (UK) Ltd.
// Copyright (C) Analog One Corporation.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.
//! Pallet to process airdrop claims

#[cfg(test)]
mod tests;

pub use pallet::*;

use polkadot_sdk::*;

use frame_support::{
	ensure,
	traits::{Currency, Get, VestingSchedule},
	weights::Weight,
	DefaultNoBound,
};
use frame_system::pallet_prelude::BlockNumberFor;
use scale_codec::Encode;
use sp_core::{
	ed25519::{Public as EdwardsPublic, Signature as EdwardsSignature},
	sr25519::{Public as SchnorrPublic, Signature as SchnorrSignature},
};
use sp_runtime::{
	traits::{CheckedSub, Verify, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	AccountId32,
};
use sp_std::{vec, vec::Vec};

type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<
	<T as frame_system::Config>::AccountId,
>>::Currency;
type BalanceOf<T> = <CurrencyOf<T> as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type RawSignature = [u8; 64];

pub trait WeightInfo {
	fn claim_raw() -> Weight;
	fn mint() -> Weight;
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn claim_raw() -> Weight {
		Weight::zero()
	}
	fn mint() -> Weight {
		Weight::zero()
	}
}

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configuration trait.
	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;
		#[pallet::constant]
		type RawPrefix: Get<&'static [u8]>;
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Someone claimed their airdrop.
		Claimed { source: AccountId32, target: T::AccountId, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account ID sending transaction has no claim.
		HasNoClaim,
		/// Provided signature is invalid.
		InvalidSignature,
		/// There's not enough in the pot to pay out some unvested amount. Generally implies a
		/// logic error.
		PotUnderflow,
		/// The account already has a vested balance.
		VestedBalanceExists,
		/// The account already has a claim associated with it
		AlreadyHasClaim,
	}

	#[pallet::storage]
	pub type Claims<T: Config> = StorageMap<_, Identity, AccountId32, BalanceOf<T>>;

	#[pallet::storage]
	pub type Total<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Vesting schedule for a claim.
	/// First balance is the total amount that should be held for vesting.
	/// Second balance is how much should be unlocked per block.
	/// The block number is when the vesting should start.
	#[pallet::storage]
	pub type Vesting<T: Config> =
		StorageMap<_, Identity, AccountId32, (BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>;

	#[pallet::genesis_config]
	#[derive(DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub claims: Vec<(AccountId32, BalanceOf<T>)>,
		#[allow(clippy::type_complexity)]
		pub vesting: Vec<(AccountId32, BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// build `Claims`
			self.claims.iter().for_each(|(a, b)| {
				Claims::<T>::insert(a, b);
			});
			// build `Total`
			Total::<T>::put(
				self.claims.iter().fold(Zero::zero(), |acc: BalanceOf<T>, &(_, b)| acc + b),
			);
			// build `Vesting`
			self.vesting.iter().for_each(|(a, b, c, d)| {
				Vesting::<T>::insert(a, (b, c, d));
			});
		}
	}

	//#[pallet::hooks]
	//impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Make a claim to collect your airdrop.
		///
		/// The dispatch origin for this call must be _None_.
		///
		/// Unsigned Validation:
		/// A call to claim is deemed valid if the signature provided matches
		/// the expected signed message of:
		///
		/// > <Bytes>(configured prefix string)(address)</Bytes>
		///
		/// and `address` matches the `destination` account.
		///
		/// Parameters:
		/// - `source`: The wallet used to register for the air-drop.
		/// - `proof`: The signature of raw signed message matching the format
		///   described above.
		/// - `target`: The destination account to payout the claim.
		///
		/// <weight>
		/// The weight of this call is invariant over the input parameters.
		/// Weight includes logic to validate unsigned `claim` call.
		///
		/// Total Complexity: O(1)
		/// </weight>
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::claim_raw())]
		pub fn claim_raw(
			origin: OriginFor<T>,
			source: AccountId32,
			proof: RawSignature,
			target: T::AccountId,
		) -> DispatchResult {
			ensure_none(origin)?;

			Self::verify_proof(&source, &proof, &target)?;
			Self::process_airdrop(source, target)?;

			Ok(())
		}

		/// Mint a new token airdrop claim.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// Parameters:
		/// - `who`: The address allowed to collect this claim.
		/// - `value`: The number of tokens that will be claimed.
		/// - `vesting_schedule`: An optional vesting schedule for these tokens.
		///
		/// <weight>
		/// The weight of this call is invariant over the input parameters.
		/// We assume worst case that both vesting and statement is being inserted.
		///
		/// Total Complexity: O(1)
		/// </weight>
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			owner: AccountId32,
			value: BalanceOf<T>,
			vesting: Option<(BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::mint_airdrop(owner, value, vesting)
		}
	}

	/// Ensure that only valid unsigned extrinsics are handled by our nodes
	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			const PRIORITY: u64 = 100;

			if let Call::claim_raw { source, proof, target } = call {
				// Check if provided proof is valid
				Self::verify_proof(source, proof, target)
					.map_err(|_| InvalidTransaction::BadProof)?;

				// Check if provided user has a valid claim
				ensure!(Claims::<T>::contains_key(source), InvalidTransaction::BadSigner);

				return Ok(ValidTransaction {
					priority: PRIORITY,
					requires: vec![],
					provides: vec![("airdrop", source).encode()],
					longevity: TransactionLongevity::MAX,
					propagate: true,
				});
			}

			// All other calls are not unsigned
			Err(InvalidTransaction::Call.into())
		}
	}
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice the length.
fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
	let mut r = Vec::with_capacity(data.len() * 2);
	let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
	for &b in data.iter() {
		push_nibble(b / 16);
		push_nibble(b % 16);
	}
	r
}

impl<T: Config> Pallet<T> {
	/// Turn target address into message to be signed as proof
	fn to_message(target: &T::AccountId) -> Vec<u8> {
		let mut message = b"<Bytes>".to_vec();
		message.extend_from_slice(T::RawPrefix::get());
		message.extend(target.using_encoded(to_ascii_hex));
		message.extend_from_slice(b"</Bytes>");
		message
	}

	/// Internal verification function that ensure a valid proof has been provided
	fn verify_proof(
		source: &AccountId32,
		proof: &RawSignature,
		target: &T::AccountId,
	) -> sp_runtime::DispatchResult {
		// Check the two supported signatures types
		let message = Self::to_message(target);
		ensure!(
			SchnorrSignature::from_raw(*proof)
				.verify(&message[..], &SchnorrPublic::from_raw(source.clone().into()))
				|| EdwardsSignature::from_raw(*proof)
					.verify(&message[..], &EdwardsPublic::from_raw(source.clone().into())),
			Error::<T>::InvalidSignature
		);

		Ok(())
	}

	/// Internal function to mint additional airdrops
	fn mint_airdrop(
		owner: AccountId32,
		value: BalanceOf<T>,
		vesting: Option<(BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>,
	) -> sp_runtime::DispatchResult {
		// Ensure to not ovewrite existing claim
		if Claims::<T>::get(&owner).is_some() {
			return Err(Error::<T>::AlreadyHasClaim.into());
		}

		// Update total, add amount and optional vesting schedule
		Total::<T>::mutate(|t| *t += value);

		Claims::<T>::insert(owner.clone(), value);
		if let Some(vs) = vesting {
			Vesting::<T>::insert(owner, vs);
		}

		Ok(())
	}

	/// Internal processing function that executes the an airdrop
	fn process_airdrop(source: AccountId32, target: T::AccountId) -> sp_runtime::DispatchResult {
		// Retreive token amount and check and update total
		let amount = Claims::<T>::get(&source).ok_or(Error::<T>::HasNoClaim)?;
		let new_total = Total::<T>::get().checked_sub(&amount).ok_or(Error::<T>::PotUnderflow)?;

		// Ensure the account has not other vesting schedules associate with it
		let vesting = Vesting::<T>::get(&source);
		if vesting.is_some() && T::VestingSchedule::vesting_balance(&target).is_some() {
			return Err(Error::<T>::VestedBalanceExists.into());
		}

		// First deposit the balance to ensure that the account exists.
		let _ = CurrencyOf::<T>::deposit_creating(&target, amount);

		// Then apply any associated vesting schedule
		if let Some(vs) = vesting {
			T::VestingSchedule::add_vesting_schedule(&target, vs.0, vs.1, vs.2)
				.expect("No other vesting schedule exists, as checked above; qed");
		}

		// Update total and remove claim
		Total::<T>::put(new_total);
		Claims::<T>::remove(&source);
		Vesting::<T>::remove(&source);

		// Deposit event on success.
		Self::deposit_event(Event::<T>::Claimed { source, target, amount });

		Ok(())
	}
}
