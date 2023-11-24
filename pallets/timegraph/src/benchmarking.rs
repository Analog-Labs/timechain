#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Timegraph;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn deposit() {
		let caller = whitelisted_caller();
		let recipient = account("recipient", 0, 1);
		let amount: BalanceOf<T> = 1_u32.into();
		#[extrinsic_call]
		deposit(RawOrigin::Signed(caller), recipient, amount);
	}

	#[benchmark]
	fn withdraw() {
		let caller = whitelisted_caller();
		let recipient = account("recipient", 0, 1);
		let amount: BalanceOf<T> = 1_u32.into();
		let sequence = 1;
		#[extrinsic_call]
		withdraw(RawOrigin::Signed(caller), recipient, amount, sequence);
	}

	impl_benchmark_test_suite!(Timegraph, crate::mock::new_test_ext(), crate::mock::Test);
}
