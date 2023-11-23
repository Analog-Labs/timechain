#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

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

	#[pallet::storage]
	#[pallet::getter(fn next_deposit_sequence)]
	pub type NextDepositSequence<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_withdrawal_sequence)]
	pub type NextWithdrawalSequence<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Deposit(T::AccountId, T::AccountId, BalanceOf<T>, u64),
		Withdrawal(T::AccountId, T::AccountId, BalanceOf<T>, u64),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// withdrawal sequence from timegraph is not expected
		WithDrawalSequenceMismatch,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The extrinsic from timegraph user to deposit funds into the timegraph account
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// deposit sequence start from 1 for each timegraph account
			NextDepositSequence::<T>::mutate(&to, |sequence| *sequence += 1);
			T::Currency::transfer(&who, &to, amount, ExistenceRequirement::KeepAlive)?;

			let deposit_sequence = Self::next_deposit_sequence(&to);
			Self::deposit_event(Event::Deposit(who, to, amount, deposit_sequence));

			Ok(())
		}

		/// The extrinsic from timegraph account to refund the token to the user
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(
			origin: OriginFor<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
			sequence: u64,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// withdrawal sequence start from 1
			let next_withdrawal_sequence = Self::next_withdrawal_sequence(&to);
			ensure!(
				sequence == next_withdrawal_sequence + 1,
				Error::<T>::WithDrawalSequenceMismatch
			);
			T::Currency::transfer(&who, &to, amount, ExistenceRequirement::KeepAlive)?;
			NextWithdrawalSequence::<T>::mutate(&who, |sequence| *sequence += 1);
			Self::deposit_event(Event::Withdrawal(who, to, amount, sequence));
			Ok(())
		}
	}
}
