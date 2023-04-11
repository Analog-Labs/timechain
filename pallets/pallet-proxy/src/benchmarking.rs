use super::*;
#[allow(unused)]
use crate::Pallet as PalletProxy;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use time_primitives::ProxyAccInput;
benchmarks! {

	set_proxy_account {
		let origin: T::AccountId = whitelisted_caller();
		let proxy_acc: T::AccountId = whitelisted_caller();
		let input = ProxyAccInput {
			max_token_usage: 10u32,
			token_usage: 10u32,
			max_task_execution: Some(10u32),
			task_executed: 10u32,
			proxy: proxy_acc,
		};

	}: _(RawOrigin::Signed(origin.clone()), input)
	verify {
		assert!( <ProxyStorage<T>>::get(origin).is_some());
	}

	impl_benchmark_test_suite!(PalletProxy, crate::mock::new_test_ext(), crate::mock::Test);
}
