#![cfg_attr(not(feature = "std"), no_std)]
//! This pallet provides functionality for users to deposit and withdraw funds.
//! It maintains a sequence for deposits and withdrawals to ensure order
//!  and prevent replay attacks.
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

#[polkadot_sdk::frame_support::pallet]
pub mod pallet {
	use polkadot_sdk::{frame_support, frame_system, sp_runtime};

	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, ExistenceRequirement, ReservableCurrency};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Saturating;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	pub trait WeightInfo {
		fn deposit() -> Weight;
		fn withdraw() -> Weight;
		fn transfer_to_pool() -> Weight;
		fn transfer_award_to_user() -> Weight;
	}

	impl WeightInfo for () {
		fn deposit() -> Weight {
			Weight::default()
		}

		fn withdraw() -> Weight {
			Weight::default()
		}

		fn transfer_to_pool() -> Weight {
			Weight::default()
		}

		fn transfer_award_to_user() -> Weight {
			Weight::default()
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: polkadot_sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as polkadot_sdk::frame_system::Config>::RuntimeEvent>;
		type WeightInfo: WeightInfo;
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		#[pallet::constant]
		type InitialThreshold: Get<BalanceOf<Self>>;
		#[pallet::constant]
		type InitialTimegraphAccount: Get<Self::AccountId>;
		#[pallet::constant]
		type InitialRewardPoolAccount: Get<Self::AccountId>;
	}

	#[pallet::type_value]
	pub fn DefaultTimegraphAccount<T: Config>() -> T::AccountId {
		T::InitialTimegraphAccount::get()
	}

	#[pallet::type_value]
	pub fn DefaultRewardPoolAccount<T: Config>() -> T::AccountId {
		T::InitialRewardPoolAccount::get()
	}

	#[pallet::type_value]
	pub fn DefaultThreshold<T: Config>() -> BalanceOf<T> {
		T::InitialThreshold::get()
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

	#[pallet::storage]
	#[pallet::getter(fn timegraph_account)]
	pub type TimegraphAccount<T: Config> =
		StorageValue<_, T::AccountId, ValueQuery, DefaultTimegraphAccount<T>>;

	#[pallet::storage]
	#[pallet::getter(fn reward_pool_account)]
	pub type RewardPoolAccount<T: Config> =
		StorageValue<_, T::AccountId, ValueQuery, DefaultRewardPoolAccount<T>>;

	#[pallet::storage]
	#[pallet::getter(fn threshold)]
	pub type Threshold<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery, DefaultThreshold<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Deposit event
		///
		/// # Parameters
		/// - `who`: The account ID of the user making the deposit.
		/// - `amount`: The amount of funds deposited.
		/// - `sequence`: The sequence number of the deposit.
		Deposit {
			who: T::AccountId,
			amount: BalanceOf<T>,
			sequence: u64,
		},
		/// Withdrawal event
		///
		/// # Parameters
		/// - `who`: The account ID of the user making the withdrawal.
		/// - `amount`: The amount of funds withdrawn.
		/// - `sequence`: The sequence number of the withdrawal.
		Withdrawal {
			who: T::AccountId,
			amount: BalanceOf<T>,
			sequence: u64,
		},

		/// Transfer to pool event
		///
		/// # Parameters
		/// - `from`: The account ID of the user transferring the funds.
		/// - `to`: The account ID of the pool receiving the funds.
		/// - `amount`: The amount of funds transferred.
		TransferToPool {
			from: T::AccountId,
			to: T::AccountId,
			amount: BalanceOf<T>,
		},

		/// Transfer award to user event
		///
		/// # Parameters
		/// - `from`: The account ID of the pool transferring the award.
		/// - `to`: The account ID of the user receiving the award.
		/// - `amount`: The amount of award transferred.
		TransferAwardToUser {
			from: T::AccountId,
			to: T::AccountId,
			amount: BalanceOf<T>,
		},

		/// Timegraph account reset event
		///
		/// # Parameters
		/// - `old`: The old timegraph account ID.
		/// - `new`: The new timegraph account ID.
		TimegraphAccountReset {
			old: T::AccountId,
			new: T::AccountId,
		},

		/// Reward pool account reset event
		///
		/// # Parameters
		/// - `old`: The old reward pool account ID.
		/// - `new`: The new reward pool account ID.
		RewardPoolAccountReset {
			old: T::AccountId,
			new: T::AccountId,
		},

		/// Threshold reset event
		///
		/// # Parameters
		/// - `old`: The old threshold value.
		/// - `new`: The new threshold value.
		ThresholdReset {
			old: BalanceOf<T>,
			new: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// sequence number overflow
		SequenceNumberOverflow,
		/// zero amount
		ZeroAmount,
		/// must keep the threshold until account is removed.
		WithdrawalAmountOverReserve,
		/// The amount to be withdrawn is not required.
		NotWithdrawalRequired,
		/// The reward pool does not have enough balance to complete the operation.
		RewardPoolOutOfBalance,
		/// The reward cannot be transferred to the same account.
		RewardToSameAccount,
		/// The new timegraph account cannot be the same as the old one.
		SameTimegraphAccount,
		/// The new reward pool account cannot be the same as the old one.
		SameRewardPoolAccount,
		/// The new threshold cannot be the same as the old one.
		SameThreshold,

		/// The sender is not a timegraph account.
		SenderIsNotTimegraph,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The extrinsic from timegraph allows user to deposit funds into the timegraph account
		///
		/// #  Flow
		/// 1. Ensure the origin is a signed account.
		/// 2. Validate the amount is greater than zero.
		/// 3. Ensure the sender .
		/// 4. Transfer the funds.
		/// 5. Increment the deposit sequence number for origin.
		/// 6. Emit a [`Event::Deposit`] event.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount > 0_u32.into(), Error::<T>::ZeroAmount);

			T::Currency::reserve(&who, amount)?;

			NextDepositSequence::<T>::try_mutate(&who, |x| -> DispatchResult {
				*x = x.checked_add(1).ok_or(Error::<T>::SequenceNumberOverflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::Deposit {
				who: who.clone(),
				amount,
				sequence: NextDepositSequence::<T>::get(who),
			});

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
		pub fn withdraw(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(amount > 0_u32.into(), Error::<T>::ZeroAmount);

			let current_reserve = T::Currency::reserved_balance(&who);
			let threshold = Threshold::<T>::get();

			ensure!(
				amount.saturating_add(threshold) <= current_reserve,
				Error::<T>::WithdrawalAmountOverReserve
			);

			ensure!(
				T::Currency::unreserve(&who, amount) == amount,
				Error::<T>::NotWithdrawalRequired
			);

			NextWithdrawalSequence::<T>::try_mutate(&who, |x| -> DispatchResult {
				*x = x.checked_add(1).ok_or(Error::<T>::SequenceNumberOverflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::Withdrawal {
				who: who.clone(),
				amount,
				sequence: NextWithdrawalSequence::<T>::get(&who),
			});
			Ok(())
		}

		/// The extrinsic from timegraph allows transferring funds to the reward pool
		///
		/// # Flow
		/// 1. Ensure the origin is the timegraph account.
		/// 2. Unreserve the specified amount from the given account.
		/// 3. Transfer the unreserved funds to the reward pool account.
		/// 4. Emit a [`Event::TransferToPool`] event.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::transfer_to_pool())]
		pub fn transfer_to_pool(
			origin: OriginFor<T>,
			account: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			Self::ensure_timegraph(origin)?;
			let unserved = T::Currency::unreserve(&account, amount);
			ensure!(unserved == amount, Error::<T>::NotWithdrawalRequired);

			T::Currency::transfer(
				&account,
				&RewardPoolAccount::<T>::get(),
				amount,
				ExistenceRequirement::KeepAlive,
			)?;

			Self::deposit_event(Event::TransferToPool {
				from: account.clone(),
				to: RewardPoolAccount::<T>::get(),
				amount,
			});

			Ok(())
		}


		/// The extrinsic from timegraph allows transferring awards to a user
		///
		/// # Flow
		/// 1. Ensure the origin is the timegraph account.
		/// 2. Ensure the account is not the reward pool account.
		/// 3. Check if the reward pool has enough balance.
		/// 4. Transfer the specified amount from the reward pool account to the given account.
		/// 5. Reserve the transferred amount in the given account.
		/// 6. Emit a [`Event::TransferAwardToUser`] event.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::transfer_award_to_user())]
		pub fn transfer_award_to_user(
			origin: OriginFor<T>,
			account: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			Self::ensure_timegraph(origin)?;
			ensure!(account != RewardPoolAccount::<T>::get(), Error::<T>::RewardToSameAccount);

			let pool_account = RewardPoolAccount::<T>::get();
			let pool_balance = T::Currency::free_balance(&pool_account);
			ensure!(pool_balance > amount, Error::<T>::RewardPoolOutOfBalance);

			T::Currency::transfer(
				&pool_account,
				&account,
				amount,
				ExistenceRequirement::KeepAlive,
			)?;

			T::Currency::reserve(&account, amount)?;

			Self::deposit_event(Event::TransferAwardToUser {
				from: RewardPoolAccount::<T>::get(),
				to: account,
				amount,
			});

			Ok(())
		}

		/// The extrinsic allows setting a new timegraph account
		///
		/// # Flow
		/// 1. Ensure the origin is the root account.
		/// 2. Ensure the new account is different from the current timegraph account.
		/// 3. Emit a [`Event::TimegraphAccountReset`] event.
		/// 4. Set the new timegraph account.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn set_timegraph_account(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				account.clone() != TimegraphAccount::<T>::get(),
				Error::<T>::SameTimegraphAccount
			);

			Self::deposit_event(Event::TimegraphAccountReset {
				old: TimegraphAccount::<T>::get(),
				new: account.clone(),
			});

			TimegraphAccount::<T>::set(account);

			Ok(())
		}

		/// The extrinsic allows setting a new reward pool account
		///
		/// # Flow
		/// 1. Ensure the origin is the root account.
		/// 2. Ensure the new account is different from the current reward pool account.
		/// 3. Emit a [`Event::RewardPoolAccountReset`] event.
		/// 4. Set the new reward pool account.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn set_reward_pool_account(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(
				account.clone() != RewardPoolAccount::<T>::get(),
				Error::<T>::SameRewardPoolAccount
			);

			Self::deposit_event(Event::RewardPoolAccountReset {
				old: RewardPoolAccount::<T>::get(),
				new: account.clone(),
			});

			RewardPoolAccount::<T>::set(account);

			Ok(())
		}

		/// The extrinsic allows setting a new threshold
		///
		/// # Flow
		/// 1. Ensure the origin is the root account.
		/// 2. Ensure the new threshold amount is different from the current threshold.
		/// 3. Emit a [`Event::ThresholdReset`] event.
		/// 4. Set the new threshold amount.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn set_threshold(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(amount != Threshold::<T>::get(), Error::<T>::SameThreshold);

			Self::deposit_event(Event::ThresholdReset {
				old: Threshold::<T>::get(),
				new: amount,
			});

			Threshold::<T>::set(amount);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensures that the origin is the current timegraph account.
		///
		/// # Parameters
		/// - `origin`: The origin of the call, which must be a signed account.
		///
		/// # Errors
		/// - Returns `Error::<T>::SenderIsNotTimegraph` if the origin is not the current timegraph account.
		pub fn ensure_timegraph(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let current_account = TimegraphAccount::<T>::get();
			ensure!(who == current_account, Error::<T>::SenderIsNotTimegraph);
			Ok(())
		}
	}
}
