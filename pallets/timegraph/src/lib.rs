#![cfg_attr(not(feature = "std"), no_std)]
//! This pallet provides functionality for users to deposit and withdraw funds. It maintains a sequence for deposits and withdrawals to ensure order and prevent replay attacks.
//!
//!
#![doc = simple_mermaid::mermaid!("../docs/timegraph_flows.mmd")]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, ExistenceRequirement};
	use frame_system::pallet_prelude::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub trait WeightInfo {
		fn deposit() -> Weight;
		fn withdraw() -> Weight;
	}

	impl WeightInfo for () {
		fn deposit() -> Weight {
			Weight::default()
		}

		fn withdraw() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Currency: Currency<Self::AccountId>;
	}

	///Stores the next deposit sequence number for each account.
	#[pallet::storage]
	#[pallet::getter(fn next_deposit_sequence)]
	pub type NextDepositSequence<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	///Stores the next withdrawal sequence number for each account.
	#[pallet::storage]
	#[pallet::getter(fn next_withdrawal_sequence)]
	pub type NextWithdrawalSequence<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Deposit event
		Deposit(T::AccountId, T::AccountId, BalanceOf<T>, u64),
		/// Withdrawal Event
		Withdrawal(T::AccountId, T::AccountId, BalanceOf<T>, u64),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// withdrawal sequence from timegraph is not expected
		WithDrawalSequenceMismatch,
		/// sequence number overflow
		SequenceNumberOverflow,
		/// zero amount
		ZeroAmount,
		/// sender same with receiver
		SenderSameWithReceiver,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The extrinsic from timegraph allows user to deposit funds into the timegraph account
		///
		/// #  Flow
		/// 1. Ensure the origin is a signed account.
		/// 2. Validate the amount is greater than zero.
		/// 3. Ensure the sender and receiver are not the same.
		/// 4. Transfer the funds.
		/// 5. Increment the deposit sequence number.
		/// 6. Emit a [`Event::Deposit`] event.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount > 0_u32.into(), Error::<T>::ZeroAmount);
			ensure!(who != to, Error::<T>::SenderSameWithReceiver);
			T::Currency::transfer(&who, &to, amount, ExistenceRequirement::KeepAlive)?;
			let deposit_sequence = Self::next_deposit_sequence(&to);
			let next_sequence =
				deposit_sequence.checked_add(1).ok_or(Error::<T>::SequenceNumberOverflow)?;
			NextDepositSequence::<T>::insert(&to, next_sequence);
			Self::deposit_event(Event::Deposit(who, to, amount, next_sequence));

			Ok(())
		}

		/// The extrinsic from timegraph allows account to refund the token to the user
		///
		/// # Flow
		/// 1. Ensure the origin is a signed account.
		/// 2. Validate the amount is greater than zero.
		/// 3. Ensure the sender and receiver are not the same.
		/// 4. Validate the withdrawal sequence number.
		/// 5. Transfer the funds.
		/// 6. Increment the withdrawal sequence number.
		/// 7. Emit a [`Event::Withdrawal`] event.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
			sequence: u64,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount > 0_u32.into(), Error::<T>::ZeroAmount);
			ensure!(who != to, Error::<T>::SenderSameWithReceiver);
			let withdrawal_sequence = Self::next_withdrawal_sequence(&who);
			let next_withdrawal_sequence =
				withdrawal_sequence.checked_add(1).ok_or(Error::<T>::SequenceNumberOverflow)?;
			ensure!(sequence == next_withdrawal_sequence, Error::<T>::WithDrawalSequenceMismatch);
			T::Currency::transfer(&who, &to, amount, ExistenceRequirement::KeepAlive)?;
			NextWithdrawalSequence::<T>::insert(&who, next_withdrawal_sequence);
			Self::deposit_event(Event::Withdrawal(who, to, amount, sequence));
			Ok(())
		}
	}
}
