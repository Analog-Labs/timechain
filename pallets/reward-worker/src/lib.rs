// This file is part of Substrate.

// Copyright (C) 2019-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Authorship tracking for FRAME runtimes.
//!
//! This tracks the current author of the block and recent uncles.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::Currency;

use sp_runtime::{traits::AtLeast32BitUnsigned, DispatchError};
use sp_std::prelude::*;
use time_primitives::WorkerTrait;

pub use pallet::*;

/// A filter on uncles which verifies seals and does no additional checks.
/// This is well-suited to consensus modes such as PoW where the cost of
/// equivocating is high.
// pub type AccountId = u128;
// T as Config>::Currency as Currency<<T as Config>::AccountId
// pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub struct SealVerify<T>(sp_std::marker::PhantomData<T>);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use crate::{types::*, weights::WeightInfo};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type KeyId = u64;
	pub type RewardList<T> = (
		// 1st account will be the rewarder
		<T as frame_system::Config>::AccountId,
		// 2nd account will be the cronical node address which gets reward
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
				num = num + 1;
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

	#[pallet::error]
	pub enum Error<T> {
		/// not in the chain.
		Invalid,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Claimed vesting.
		BalanceAmount { who: T::AccountId, amount: BalanceOf<T> },
	}
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// in progress
		#[pallet::weight(0)]
		pub fn submit_reward(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			// Retrieve sender of the transaction.
			let who = ensure_signed(origin)?;
			// Add the reward to the on-chain list.
			let data = T::Currency::total_balance(&who);
			// data
			Self::deposit_event(Event::BalanceAmount { who, amount: data });
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		// fn get_balance(who: &T::AccountId) -> Result<BalanceOf<T>, DispatchError> {
		// 	let data = T::Currency::total_balance(&who);
		// 	Ok(data)
		// }

		fn get_reward_account() -> Result<Vec<(T::AccountId, T::AccountId)>, DispatchError> {
			let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			Ok(data_list)
		}

		fn div_balance<N>(value: N, q: u32) -> N
		where
			N: AtLeast32BitUnsigned + Clone,
		{
			let result_div = value.clone() / q.into();

			result_div
		}
		pub fn send_reward(
			balance: BalanceOf<T>,
			curr: Vec<T::AccountId>,
		) -> Result<(), DispatchError> {
			// let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			let mut length: u32 = 0;
			curr.iter().for_each(|_| {
				length = length + 1;
			});

			let balance_paid = Self::div_balance(balance, length);
			curr.iter().for_each(|item| {
				log::info!("Balance transfered ====> {:?} --- amount= {:?} ", item, balance_paid);
				let _resp = T::Currency::deposit_into_existing(&item, balance_paid);
			});

			Ok(())
		}
		fn insert_account(
			validator: T::AccountId,
		) -> Result<(T::AccountId, T::AccountId), DispatchError> {
			let data_list = RewardAccount::<T>::iter_values().collect::<Vec<_>>();
			let mut length: u64 = 0;
			data_list.iter().for_each(|_| {
				length = length + 1;
			});

			RewardAccount::<T>::insert(length + 1, (validator.clone(), validator.clone()));

			Ok((validator.clone(), validator.clone()))
		}
	}
	impl<T: Config> WorkerTrait<T::AccountId, BalanceOf<T>> for Pallet<T> {
		fn get_reward_acc() -> Result<Vec<(T::AccountId, T::AccountId)>, DispatchError> {
			Self::get_reward_account()
		}

		fn send_reward_to_acc(
			balance: BalanceOf<T>,
			curr: Vec<T::AccountId>,
		) -> Result<(), DispatchError> {
			Self::send_reward(balance.into(), curr)
		}

		fn insert_validator(
			validator: T::AccountId,
		) -> Result<(T::AccountId, T::AccountId), DispatchError> {
			let result = Self::insert_account(validator);

			result
		}
	}
}
