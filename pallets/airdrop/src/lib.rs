#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]
//! Pallet to process airdrop claims

#[cfg(feature = "runtime-benchmarks")]
mod benchmarks;
#[cfg(test)]
mod mock;
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
	traits::{CheckedSub, Saturating, Verify, Zero},
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
	fn transfer() -> Weight;
}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn claim_raw() -> Weight {
		Weight::zero()
	}
	fn mint() -> Weight {
		Weight::zero()
	}
	fn transfer() -> Weight {
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
		/// Vesting backend used to provided vested airdrops.
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;
		/// Prefix to include in message signed by user to clarify intend.
		#[pallet::constant]
		type RawPrefix: Get<&'static [u8]>;
		/// Additional safety check to avoid, minting claims below existential balance.
		type MinimumBalance: Get<BalanceOf<Self>>;
		/// Weight information of the pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new airdrop eligibility has been created.
		Minted { owner: AccountId32, amount: BalanceOf<T> },
		/// An airdrop was moved to a new owner
		Moved { from: AccountId32, to: AccountId32 },
		/// Someone claimed their airdrop.
		Claimed { source: AccountId32, target: T::AccountId, amount: BalanceOf<T> },
	}

	#[derive(PartialEq)]
	#[pallet::error]
	pub enum Error<T> {
		/// Account ID sending transaction has no claim.
		HasNoClaim,
		/// Provided signature is invalid.
		InvalidSignature,
		/// Claimed exceed minted funds, implies internal logic error.
		PotUnderflow,
		/// The size of the airdrop is below the minimal allowed amount
		BalanceTooSmall,
		/// The account already has to many vesting schedule associated with it.
		VestingNotPossible,
		/// The account already has a claim associated with it
		AlreadyHasClaim,
	}

	/// List of all unclaimed airdrops and their amounts
	#[pallet::storage]
	pub type Claims<T: Config> = StorageMap<_, Identity, AccountId32, BalanceOf<T>>;

	/// Total sum of all unclaimed airdrops
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
		/// > `<Bytes>(configured prefix string)(address)</Bytes>`
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

			Ok(Self::mint_airdrop(owner, value, vesting)?)
		}

		/// Transfer a token airdrop claim to a new owner.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// Parameters:
		/// - `from`: The address from which to move the claim.
		/// - `to`: The address to which to move the claim. Only addresses with no existing claims are allowed.
		///
		/// <weight>
		/// The weight of this call is invariant over the input parameters.
		/// We assume worst case that claim and vesting needs to be moved.
		///
		/// Total Complexity: O(1)
		/// </weight>
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			from: AccountId32,
			to: AccountId32,
		) -> DispatchResult {
			ensure_root(origin)?;

			Ok(Self::move_airdrop(from, to)?)
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
	pub fn mint_airdrop(
		owner: AccountId32,
		amount: BalanceOf<T>,
		vesting: Option<(BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>,
	) -> Result<(), Error<T>> {
		// Ensure amount is large enough and mint does not overwrite existing claim
		ensure!(amount >= T::MinimumBalance::get(), Error::<T>::BalanceTooSmall);
		ensure!(Claims::<T>::get(&owner).is_none(), Error::<T>::AlreadyHasClaim);

		// Update total, add amount and optional vesting schedule
		Total::<T>::mutate(|t| *t += amount);

		Claims::<T>::insert(&owner, amount);
		if let Some(vs) = vesting {
			Vesting::<T>::insert(owner.clone(), vs);
		}

		// Deposit event on success.
		Self::deposit_event(Event::<T>::Minted { owner, amount });

		Ok(())
	}

	/// Internal function to mint additional airdrops, adding to existing airdrops if necessary
	pub fn add_airdrop(
		owner: AccountId32,
		amount: BalanceOf<T>,
		vesting: Option<(BalanceOf<T>, BalanceOf<T>, BlockNumberFor<T>)>,
	) -> Result<(), Error<T>> {
		// Ensure amount is large enough and vesting can be accurately represented claim
		ensure!(amount >= T::MinimumBalance::get(), Error::<T>::BalanceTooSmall);
		ensure!(
			vesting.is_none() || Vesting::<T>::get(&owner).is_none(),
			Error::<T>::VestingNotPossible
		);

		// Update total, add amount and optional vesting schedule
		Total::<T>::mutate(|t| *t = *t.saturating_add(amount));

		let combined = amount.saturating_add(Claims::<T>::take(&owner).unwrap_or_default());
		Claims::<T>::insert(&owner, combined);
		if let Some(vs) = vesting {
			debug_assert!(Vesting::<T>::get(&owner).is_none());
			Vesting::<T>::insert(&owner, vs);
		}

		// Deposit event on success.
		Self::deposit_event(Event::<T>::Minted { owner, amount });

		Ok(())
	}

	/// Internal function to mint additional airdrops
	pub fn move_airdrop(from: AccountId32, to: AccountId32) -> Result<(), Error<T>> {
		// Check to and from are valid
		ensure!(Claims::<T>::get(&to).is_none(), Error::<T>::AlreadyHasClaim);
		if let Some(amount) = Claims::<T>::take(&from) {
			// Move claim and its vesting schedule
			Claims::<T>::insert(&to, amount);
			if let Some(vesting) = Vesting::<T>::take(&from) {
				Vesting::<T>::insert(&to, vesting);
			}

			// Deposit event on success.
			Self::deposit_event(Event::<T>::Moved { from, to });

			Ok(())
		} else {
			Err(Error::<T>::HasNoClaim)
		}
	}

	/// Internal processing function that executes an airdrop
	fn process_airdrop(source: AccountId32, target: T::AccountId) -> sp_runtime::DispatchResult {
		// Retrieve token amount and check and update total
		let amount = Claims::<T>::get(&source).ok_or(Error::<T>::HasNoClaim)?;
		let new_total = Total::<T>::get().checked_sub(&amount).ok_or(Error::<T>::PotUnderflow)?;

		// Ensure the account has not other vesting schedules associate with it
		let vesting = Vesting::<T>::get(&source);
		ensure!(
			vesting.is_none()
				|| T::VestingSchedule::can_add_vesting_schedule(
					&target,
					vesting.expect("Vesting is some; qed").0,
					vesting.expect("Vesting is some; qed").1,
					vesting.expect("Vesting is some; qed").2,
				)
				.is_ok(),
			Error::<T>::VestingNotPossible
		);

		// First deposit the balance to ensure that the account exists.
		let _ = CurrencyOf::<T>::deposit_creating(&target, amount);

		// Then apply any associated vesting schedule
		if let Some(vs) = vesting {
			T::VestingSchedule::add_vesting_schedule(&target, vs.0, vs.1, vs.2)
				.expect("Adding vesting schedule was checked above; qed");
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
