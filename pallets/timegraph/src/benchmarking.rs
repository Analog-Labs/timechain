#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Timegraph;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_support::traits::Currency;
	use scale_info::prelude::vec;

	#[benchmark]
	fn deposit() {
		let caller = whitelisted_caller();
		let recipient = account("recipient", 0, 1);
		let amount: BalanceOf<T> = 500_000_000_u32.into();
		let amount_be: BalanceOf<T> = 1_000_000_000_u32.into();
		T::Currency::make_free_balance_be(&caller, amount_be);
		#[extrinsic_call]
		deposit(RawOrigin::Signed(caller), recipient, amount);
	}

	#[benchmark]
	fn withdraw() {
		let caller = whitelisted_caller();
		let recipient = account("recipient", 0, 1);
		let amount: BalanceOf<T> = 500_000_000_u32.into();
		let sequence = 1;
		let amount_be: BalanceOf<T> = 1_000_000_000_u32.into();
		T::Currency::make_free_balance_be(&caller, amount_be);
		#[extrinsic_call]
		withdraw(RawOrigin::Signed(caller), recipient, amount, sequence);
	}

	impl_benchmark_test_suite!(Timegraph, crate::mock::new_test_ext(), crate::mock::Test);
}
