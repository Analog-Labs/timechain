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

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	dispatch,
	traits::{Defensive, FindAuthor, Get, VerifySeal, Currency},
	BoundedSlice, BoundedVec,
};
// use sp_authorship::{InherentError, UnclesInherentData, INHERENT_IDENTIFIER};
use sp_runtime::{traits::{Header as HeaderT, One, Saturating, UniqueSaturatedInto, StaticLookup}, AccountId32, DispatchResult, DispatchError};
use sp_std::{collections::btree_set::BTreeSet, prelude::*, result};

pub use pallet::*;


/// A filter on uncles which verifies seals and does no additional checks.
/// This is well-suited to consensus modes such as PoW where the cost of
/// equivocating is high.
/// 
// pub type AccountId = u128;
// T as Config>::Currency as Currency<<T as Config>::AccountId
// pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub struct SealVerify<T>(sp_std::marker::PhantomData<T>);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use crate::{types::*, weights::WeightInfo};
	use frame_support::{pallet_prelude::*, traits::OffchainWorker};
	use frame_system::pallet_prelude::*;

	pub(crate) type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type key_id = u64;
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
			/// default implimentation
			GenesisConfig { reward_list: vec![] }
		}
	}
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// will add additional locgic if needed
			/// in-progress on each finality grab reward from validator account
			/// will apply some checks
			log::info!("Hello World from offchain workers build list of reward accounts! {:?}", self.reward_list);
			let inout = self.reward_list[0].clone();
			RewardAccount::<T>::insert(1,inout);
			let data = T::Currency::total_balance(&self.reward_list[0].0);
			log::info!("Balance of 1st account {:?}",data);

		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			
			log::info!("Hello World from on initialize!");
			/// in-progress on each finality grab reward from validator account
			/// 
			Weight::zero()
		}

		fn on_finalize(_block_number: T::BlockNumber) {
			log::info!("Hello World from on finalize!");

			// if sp_io::offchain::is_validator() {
				// let balance = T::Currency::total_balance(ALICE);
				let prev_block = PrevBlockNumber::<T>::get(1);
				match prev_block  {
					None => {
						PrevBlockNumber::<T>::insert(1, _block_number);
					},
					Some(block_no) => {
						if _block_number > block_no  &&  _block_number != block_no {
							let data_list = RewardAccount::<T>::get(1);
							match data_list  {
								None => {
									log::info!("Hello World from offchain workers! balance from ====> error accored ");
								},
								Some(vals) => {
									let balance1 = T::Currency::total_balance(&vals.0);
									let balance2 = T::Currency::total_balance(&vals.1);
									log::info!("Hello World from offchain workers! balance from ====>  {:?} ---- {:?}", balance1, balance2);
									// reward from the validators account
									let val = T::Currency::transfer(&vals.0, &vals.1, balance1,frame_support::traits::ExistenceRequirement::AllowDeath );
									// mint reward and transfer it
									let val2 = T::Currency::deposit_into_existing(&vals.0,balance2);
									
									match val {
										Ok(res) => {
											PrevBlockNumber::<T>::insert(1,_block_number);
											log::info!("tokens transfered");
										},
										Err(e) => {
											log::info!(" something went wrong  {:?}", e);
										}
									}
								}
							}
						}
					}
				}
				log::info!("its a validator {:?}", _block_number);
				
				
			// }
		}

		fn offchain_worker(_block_number: T::BlockNumber) {
			// Note that having logs compiled to WASM may cause the size of the blob to increase
			// significantly. You can use `RuntimeDebug` custom derive to hide details of the types
			// in WASM. The `sp-api` crate also provides a feature `disable-logging` to disable
			// all logging and thus, remove any logging from the WASM.
			// let data = T::Currency::total_balance(&who);
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn reward_accounts)]
	pub(super) type RewardAccount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		key_id,
		RewardList<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn prev_block)]
	pub(super) type PrevBlockNumber<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		key_id,
		T::BlockNumber,
		OptionQuery,
	>;

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
	
}


impl<T: Config> Pallet<T> {
	/// Fetch the author of the block.
	///
	/// This is safe to invoke in `on_initialize` implementations, as well
	/// as afterwards.
	fn get_balance(who: &T::AccountId) -> Result<BalanceOf<T>, DispatchError> {
		// Check the memoized storage value.
		// if let Some(author) = <Author<T>>::get() {
			
			let data = T::Currency::total_balance(&who);
		// }
		Ok(data)

	}

	
}