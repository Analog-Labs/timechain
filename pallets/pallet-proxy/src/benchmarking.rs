use super::*;
#[allow(unused)]
use crate::Pallet as PalletProxy;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::SaturatedConversion;
use time_primitives::{ProxyAccStatus, ProxyStatus};
benchmarks! {

	set_proxy_account {
		let origin: T::AccountId = whitelisted_caller();
		let proxy_acc: T::AccountId = whitelisted_caller();

	}: _(RawOrigin::Signed(origin.clone()), Some(10u32.into()), 10u32.into(), Some(10u32), 10u32, proxy_acc)
	verify {
		assert!(<ProxyStorage<T>>::get(origin).is_some());
	}

	update_proxy_account {
		let origin: T::AccountId = whitelisted_caller();
		let proxy_acc: T::AccountId = whitelisted_caller();
		let (max_token_usage, token_usage, max_task_execution, task_executed, proxy) =  (
			Some(10u32),
			10u32,
			Some(10u32),
			10u32,
			proxy_acc,
		);

		let data = ProxyAccStatus {
			owner: origin.clone(),
			max_token_usage: max_token_usage.as_ref().map(|&x| x.saturated_into()),
			token_usage: token_usage.saturated_into(),
			max_task_execution,
			task_executed,
			status: ProxyStatus::Valid,
			proxy: proxy.clone(),
		};
		let _ = <ProxyStorage<T>>::insert(origin.clone(),data);

		let output_expected = ProxyAccStatus {
			owner: origin.clone(),
			max_token_usage: max_token_usage.as_ref().map(|&x| x.saturated_into::<BalanceOf<T>>()),
			token_usage: token_usage.saturated_into(),
			max_task_execution,
			task_executed,
			status: ProxyStatus::Suspended,
			proxy: proxy.clone(),
		};

	}: _(RawOrigin::Signed(origin.clone()), proxy, ProxyStatus::Suspended)
	verify {
		assert_eq!( <ProxyStorage<T>>::get(origin).unwrap(), output_expected);
	}

	remove_proxy_account {
		let origin: T::AccountId = whitelisted_caller();
		let proxy_acc: T::AccountId = whitelisted_caller();
		let (max_token_usage, token_usage, max_task_execution, task_executed, proxy) =  (
			Some(10u32),
			10u32,
			Some(10u32),
			10u32,
			proxy_acc,
		);

		let data = ProxyAccStatus {
			owner: origin.clone(),
			max_token_usage: max_token_usage.as_ref().map(|&x| x.saturated_into()),
			token_usage: token_usage.saturated_into(),
			max_task_execution: max_task_execution,
			task_executed,
			status: ProxyStatus::Valid,
			proxy: proxy.clone(),
		};
		let _ = <ProxyStorage<T>>::insert(origin.clone(),data);

	}: _(RawOrigin::Signed(origin.clone()), proxy)
	verify {
		assert!( <ProxyStorage<T>>::get(origin).is_none());
	}

	impl_benchmark_test_suite!(PalletProxy, crate::mock::new_test_ext(), crate::mock::Test);
}
