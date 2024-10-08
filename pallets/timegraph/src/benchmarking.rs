#![cfg(feature = "runtime-benchmarks")]
#![allow(clippy::duplicated_attributes)]
use super::*;

#[allow(unused)]
use crate::Pallet as Timegraph;

use polkadot_sdk::{frame_benchmarking, frame_support, frame_system};

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_support::traits::Currency;

	#[benchmark]
	fn deposit() {
		let caller = whitelisted_caller();
		let amount: BalanceOf<T> = 5_000_000u32.into();
		let amount_be: BalanceOf<T> = amount * 100u32.into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount_be));
		#[extrinsic_call]
		deposit(RawOrigin::Signed(caller), amount);
	}

	#[benchmark]
	fn withdraw() {
		let caller = whitelisted_caller();
		let amount: BalanceOf<T> = 5_000_000u32.into();
		let amount_be: BalanceOf<T> = amount * 100u32.into();
		T::Currency::resolve_creating(&caller, T::Currency::issue(amount_be));
		#[extrinsic_call]
		withdraw(RawOrigin::Signed(caller), amount);
	}

	#[benchmark]
	fn transfer_to_pool() {
		let caller: T::AccountId = whitelisted_caller();
		let account: T::AccountId = account("name", 0, 0);
		let amount: BalanceOf<T> = 5_000_000u32.into();
		#[extrinsic_call]
		// Example new account
		transfer_to_pool(RawOrigin::Signed(caller), account, amount);
	}

	// #[benchmark]
	// fn set_timegraph_account() {
	// 	let caller: T::AccountId = whitelisted_caller();
	// 	let new_account: T::AccountId = T::AccountId::from([0u8; 32]);
	// 	#[extrinsic_call]
	// 	// Example new account
	// 	withdraw(RawOrigin::Signed(caller), amount);
	// }

	// // Benchmark for the `set_reward_pool_account` extrinsic
	// #[benchmark]
	// fn set_reward_pool_account() {
	//     let caller: T::AccountId = whitelisted_caller();
	//     let new_pool_account: T::AccountId = T::AccountId::from([1u8; 32]); // Example new pool account
	// }: _(RawOrigin::Root, new_pool_account)
	// verify {
	//     // Verify that the new reward pool account is set correctly
	//     assert_eq!(RewardPoolAccount::<T>::get(), new_pool_account);
	// }

	// // Benchmark for the `transfer_award_to_user` extrinsic
	// #[benchmark]
	// fn transfer_award_to_user() {
	//     let caller: T::AccountId = whitelisted_caller();
	//     let user_account: T::AccountId = T::AccountId::from([2u8; 32]); // Example user account
	//     let amount: BalanceOf<T> = 100u32.into(); // Example transfer amount

	//     // Ensure the reward pool has enough balance
	//     let reward_pool_account = RewardPoolAccount::<T>::get();
	//     T::Currency::make_free_balance_be(&reward_pool_account, 1000u32.into());
	// }: _(RawOrigin::Signed(caller), user_account, amount)
	// verify {
	//     // Verify that the user's balance is updated correctly
	//     assert_eq!(T::Currency::free_balance(&user_account), amount);
	// }

	impl_benchmark_test_suite!(Timegraph, crate::mock::new_test_ext(), crate::mock::Test);
}
