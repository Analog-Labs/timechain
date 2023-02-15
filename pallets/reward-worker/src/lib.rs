//! Authorship tracking for FRAME runtimes.
//!
//! This tracks the current author of the block and recent uncles.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;

use sp_runtime::{DispatchError, SaturatedConversion};
use sp_std::prelude::*;
use time_primitives::WorkerTrait;

pub use pallet::*;
use pallet_session::{self as sessions};
use pallet_staking::{self as staking, EraPayout, SessionInterface};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::BlockNumberFor;
	use pallet_session::ShouldEndSession;

	use log::info;
	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type KeyId = u64;
	pub type BalanceCur<T> = <T as staking::Config>::CurrencyBalance;
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
	pub trait Config: frame_system::Config + staking::Config + sessions::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Find the author of a block.
		type Currency: Currency<Self::AccountId>;
		// type Worker: WorkerTrait<Self::AccountId>;
		type ShouldEndSession: ShouldEndSession<Self::BlockNumber>;
		type SessionInterface: SessionInterface<Self::AccountId>;

		type EraPayout: EraPayout<BalanceCur<Self>>;
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

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Called when a block is initialized. Will rotate session if it is the last
		/// block of the current session.
		fn on_initialize(n: T::BlockNumber) -> Weight {
			// check if session is about to end
			if <T as pallet::Config>::ShouldEndSession::should_end_session(n) {
				info!(" ---->>>>>new session happened");

				// let da = <T as pallet::Config>::SessionInterface::validators();
				// let active = <staking::Pallet<T>>::active_era();
				// let sess = <sessions::Pallet<T>>::current_index();
				// match active {
				// 	Some(val) => {
				// 		let now_as_millis_u64 = T::UnixTime::now().as_millis().saturated_into::<u64>();

				// 		let era_duration = (now_as_millis_u64).saturated_into::<u64>();

				// 		let staked = <staking::Pallet<T>>::eras_total_stake(val.index);
				// 		let issuance = <T as staking::Config>::Currency::total_issuance();
				// 		info!(" ---->>>>>staked amount {:?}", staked);
				// 		// era_duration
				// 		// let balance_staked:BalanceOf = Self::into_fun(staked);
				// 		let (validator_payout, remainder) = <T as
				// pallet::Config>::EraPayout::era_payout(staked, issuance, era_duration);

				// 		// let percent_from_validator = Self::percent_calculator(validator_payout, 20);
				// 		// let percent_from_remainder = Self::percent_calculator(remainder, 20);

				// 		// let validator_share = validator_payout.saturating_sub(percent_from_validator);
				// 		// let remainder_share = remainder.saturating_sub(percent_from_remainder);
				// 		// let cronical_share =
				// percent_from_validator.saturating_add(percent_from_remainder);

				// 	}
				// 	None => {}
				// }

				Weight::zero()
			} else {
				// 	// NOTE: the non-database part of the weight for `should_end_session(n)` is
				// 	// included as weight for empty block, the database part is expected to be in
				// 	// cache.
				Weight::zero()
			}
		}
	}
	impl<T: Config> Pallet<T> {
		pub fn get_reward_account() -> Result<WorkerReturn<T::AccountId>, DispatchError> {
			let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			Ok(data_list)
		}

		pub fn send_reward(
			balance: BalanceOf<T>,
			curr: Vec<T::AccountId>,
		) -> Result<(), DispatchError> {
			let length = curr.len().saturated_into::<u32>();
			if length != 0 {
				let balance_paid = balance / length.into();

				curr.iter().for_each(|item| {
					log::info!(
						"Balance transferred. ====> {:?} --- amount= {:?} ",
						item,
						balance_paid
					);
					let _resp =
						<T as pallet::Config>::Currency::deposit_into_existing(item, balance_paid);
				});
			}

			Ok(())
		}

		pub fn insert_account(validator: T::AccountId) -> Result<T::AccountId, DispatchError> {
			let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			let length = data_list.len().saturated_into::<u64>();

			RewardAccount::<T>::insert(length + 1, (validator.clone(), validator.clone()));

			Ok(validator)
		}

		pub fn percent_calculator<N>(value: N, q: u32) -> N
		where
			N: sp_runtime::traits::AtLeast32BitUnsigned + Clone,
		{
			let divisor = 100u32;
			if q == 0 {
				return value;
			}

			(value / q.into()).saturating_mul(divisor.into())
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

	// impl<T: Config> pallet_session::SessionHandler<T::AccountId> for Pallet<T> {

	// 	fn on_before_session_ending() {
	// 		info!("zain planning new session -->>");
	// 	}
	// }
}
