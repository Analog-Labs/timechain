//! Authorship tracking for FRAME runtimes.
//!
//! This tracks the current author of the block and recent uncles.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;

use sp_runtime::{traits::AtLeast32BitUnsigned, DispatchError};
use sp_std::prelude::*;
use time_primitives::WorkerTrait;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type KeyId = u64;
	pub type WorkerReturn<AccountId> = Vec<(AccountId, AccountId)>;
	pub type RewardList<T> = (
		// 1st account will be the rewarder
		<T as frame_system::Config>::AccountId,
		// 2nd account will be the chronicle node address which gets reward
		<T as frame_system::Config>::AccountId,
	);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Find the author of a block.
		type Currency: Currency<Self::AccountId>;
		// type Worker: WorkerTrait<Self::AccountId>;
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// get reward from one account and transfer to cronical node account
		/// pair of accounts which gives and takes rewards
		pub reward_list: Vec<RewardList<T>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { reward_list: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			log::info!("build list of reward accounts! {:?}", self.reward_list);
			let mut num: u64 = 0;
			self.reward_list.iter().for_each(|item| {
				num += 1;
				RewardAccount::<T>::insert(num, item.clone());
			});
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn reward_accounts)]
	pub(super) type RewardAccount<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, RewardList<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn prev_block)]
	pub(super) type PrevBlockNumber<T: Config> =
		StorageMap<_, Blake2_128Concat, KeyId, T::BlockNumber, OptionQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {
		/// Claimed vesting.
		BalanceAmount { who: T::AccountId, amount: BalanceOf<T> },
	}

	impl<T: Config> Pallet<T> {
		fn get_reward_account() -> Result<WorkerReturn<T::AccountId>, DispatchError> {
			let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			Ok(data_list)
		}

		fn div_balance<N>(value: N, q: u32) -> Option<N>
		where
			N: AtLeast32BitUnsigned + Clone,
		{
			if q > 0 {
				Some(value / q.into())
			} else {
				None
			}
		}

		pub fn send_reward(
			balance: BalanceOf<T>,
			curr: Vec<T::AccountId>,
		) -> Result<(), DispatchError> {
			let mut length: u32 = 0;
			curr.iter().for_each(|_| {
				length += 1;
			});

			let balance_paid_opt = Self::div_balance(balance, length);
			match balance_paid_opt {
				Some(balance_paid) => {
					curr.iter().for_each(|item| {
						log::info!(
							"Balance transferred. ====> {:?} --- amount= {:?} ",
							item,
							balance_paid
						);
						let _resp = T::Currency::deposit_into_existing(item, balance_paid);
					});
				},
				None => {
					log::info!("invalid Balance value");
				},
			}

			Ok(())
		}
		fn insert_account(validator: T::AccountId) -> Result<T::AccountId, DispatchError> {
			let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			let mut length: u64 = 0;
			data_list.iter().for_each(|_| {
				length += 1;
			});

			RewardAccount::<T>::insert(length + 1, (validator.clone(), validator.clone()));

			Ok(validator)
		}
	}
	impl<T: Config> WorkerTrait<T::AccountId, BalanceOf<T>> for Pallet<T> {
		fn get_reward_acc() -> Result<WorkerReturn<T::AccountId>, DispatchError> {
			Self::get_reward_account()
		}

		fn send_reward_to_acc(
			balance: BalanceOf<T>,
			curr: Vec<T::AccountId>,
		) -> Result<(), DispatchError> {
			Self::send_reward(balance, curr)
		}

		fn insert_validator(validator: T::AccountId) -> Result<T::AccountId, DispatchError> {
			Self::insert_account(validator)
		}
	}
}
