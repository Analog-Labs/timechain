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
		let recipient = account("recipient", 0, 1);
		let amount: BalanceOf<T> = 5_000_000u32.into();
		let amount_be: BalanceOf<T> = amount * 100u32.into();
		T::Currency::make_free_balance_be(&caller, amount_be);
		#[extrinsic_call]
		deposit(RawOrigin::Signed(caller), recipient, amount);
	}

	#[benchmark]
	fn withdraw() {
		let caller = whitelisted_caller();
		let recipient = account("recipient", 0, 1);
		let amount: BalanceOf<T> = 5_000_000u32.into();
		let amount_be: BalanceOf<T> = amount * 100u32.into();
		let sequence = 1;
		T::Currency::make_free_balance_be(&caller, amount_be);
		#[extrinsic_call]
		withdraw(RawOrigin::Signed(caller), recipient, amount, sequence);
	}

	impl_benchmark_test_suite!(Timegraph, crate::mock::new_test_ext(), crate::mock::Test);
}
