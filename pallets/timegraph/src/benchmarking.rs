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
	use frame_support::traits::{Currency, ReservableCurrency};

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
		let _ = T::Currency::reserve(&caller, Threshold::<T>::get() + amount);

		#[extrinsic_call]
		withdraw(RawOrigin::Signed(caller), amount);
	}

	#[benchmark]
	fn transfer_to_pool() {
		let caller: T::AccountId = whitelisted_caller();
		let account: T::AccountId = account("name", 0, 0);
		let amount: BalanceOf<T> = 5_000_000_u32.into();
		T::Currency::resolve_creating(
			&account,
			T::Currency::issue(amount * 100u32.into() + Threshold::<T>::get()),
		);
		TimegraphAccount::<T>::set(caller.clone());
		let _ = T::Currency::reserve(&account, Threshold::<T>::get() + amount);

		#[extrinsic_call]
		transfer_to_pool(RawOrigin::Signed(caller), account, amount);
	}

	#[benchmark]
	fn transfer_award_to_user() {
		let caller: T::AccountId = whitelisted_caller();
		let account: T::AccountId = account("name", 0, 0);
		let amount: BalanceOf<T> = 5_000_000_u32.into();

		T::Currency::resolve_creating(&caller, T::Currency::issue(amount * 200u32.into()));

		T::Currency::resolve_creating(&account, T::Currency::issue(amount * 100u32.into()));

		TimegraphAccount::<T>::set(caller.clone());
		RewardPoolAccount::<T>::set(caller.clone());

		#[extrinsic_call]
		transfer_award_to_user(RawOrigin::Signed(caller), account, amount);
	}

	#[benchmark]
	fn set_timegraph_account() {
		let new_account: T::AccountId = account("name", 0, 0);
		#[extrinsic_call]
		// Example new account
		set_timegraph_account(RawOrigin::Root, new_account);
	}

	#[benchmark]
	fn set_reward_pool_account() {
		let new_account: T::AccountId = account("name", 0, 0);

		#[extrinsic_call]
		set_reward_pool_account(RawOrigin::Root, new_account);
	}

	#[benchmark]
	fn set_threshold() {
		let amount: BalanceOf<T> = 5_000_000_u32.into();

		#[extrinsic_call]
		set_threshold(RawOrigin::Root, amount);
	}

	impl_benchmark_test_suite!(Timegraph, crate::mock::new_test_ext(), crate::mock::Test);
}
