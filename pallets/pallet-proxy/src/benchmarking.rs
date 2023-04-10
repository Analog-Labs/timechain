use super::*;
#[allow(unused)]
use crate::Pallet as PalletProxy;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::string::String;
use time_primitives::{ProxyAccInput, ProxyAccStatus, ProxyStatus};

benchmarks! {

	set_delegate_account {
		let origin: T::AccountId = whitelisted_caller();
		let input  = ProxyAccInput {
			max_token_usage: 10,
			token_usage: 10,
			max_task_execution: 10
			task_executed: 10,
		},

		let proxy = input.clone();
	}: _(RawOrigin::Signed(origin.clone()), proxy)
	verify {
		assert!( <ProxyStorage<T>>::get(origin).is_some());
	}

	impl_benchmark_test_suite!(PalletProxy, crate::mock::new_test_ext(), crate::mock::Test);
}
