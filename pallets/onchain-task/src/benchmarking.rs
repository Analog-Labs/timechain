use super::*;

#[allow(unused)]
use crate::Pallet as OnChainTask;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_std::borrow::ToOwned;
use scale_info::prelude::format;
use crate::{types::*};
use sp_std::prelude::*;

benchmarks! {


	store_task {

		let s in 0 .. 100;

		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;
		let mut task = Vec::new();
		let mut task_methods = Vec::new();
		let task_method = TaskMethod {
			name: format!("{}{}", "this_is_the_chain_method", s).as_bytes().to_owned(),
			arguments: format!("{}{}","this_is_the_chain_arguments", s).as_bytes().to_owned(),
		};
		task_methods.push(task_method);
		let onchain_task = TaskData {
			task:  SupportedTasks::SwapToken,
			task_data: format!("{}{}","this_is_the_chain_task", s).as_bytes().to_owned(),
			method: task_methods.clone(),
		};
		task.push(onchain_task);
		let mut task_data = Vec::new();
		let chain_tasks = OnchainTasks{
			task,
		};
		task_data.push(chain_tasks.clone());

	}: _(RawOrigin::Signed(who), chain.clone(), chain_tasks.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain.clone()), Some(task_data.clone()));
	}

	edit_task {

		let s in 0 .. 100;

		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;
		let mut task = Vec::new();
		let mut task2 = Vec::new();
		let mut task_methods = Vec::new();
		let mut task_methods2 = Vec::new();
		let task_method = TaskMethod {
			name: format!("{}{}", "this_is_the_chain_method", s).as_bytes().to_owned(),
			arguments: format!("{}{}","this_is_the_chain_arguments", s).as_bytes().to_owned(),
		};
		let task_method2 = TaskMethod {
			name: format!("{}{}", "this_is_the_chain_method1", s).as_bytes().to_owned(),
			arguments: format!("{}{}","this_is_the_chain_arguments1", s).as_bytes().to_owned(),
		};
		task_methods.push(task_method);
		task_methods2.push(task_method2);
		let onchain_task = TaskData {
			task:  SupportedTasks::SwapToken,
			task_data: format!("{}{}","this_is_the_chain_task", s).as_bytes().to_owned(),
			method: task_methods.clone(),
		};
		let onchain_task2 = TaskData {
			task:  SupportedTasks::SwapToken,
			task_data: format!("{}{}","this_is_the_chain_task", s).as_bytes().to_owned(),
			method: task_methods.clone(),
		};
		task.push(onchain_task);
		task2.push(onchain_task2);
		let mut task_data = Vec::new();
		let mut chain_task2 = Vec::new();
		let chain_tasks = OnchainTasks{
			task,
		};
		let chain_tasks2 = OnchainTasks{
			task: task2,
		};
		task_data.push(chain_tasks.clone());
		chain_task2.push(chain_tasks2.clone());

	}: _(RawOrigin::Signed(who), chain.clone(), chain_tasks.clone(), chain_tasks2.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain.clone()), Some(task_data.clone()));
	}

	remove_single_task {

		let s in 0 .. 100;

		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;
		let mut task = Vec::new();
		let mut task_methods = Vec::new();
		let task_method = TaskMethod {
			name: format!("{}{}", "this_is_the_chain_method", s).as_bytes().to_owned(),
			arguments: format!("{}{}","this_is_the_chain_arguments", s).as_bytes().to_owned(),
		};
		task_methods.push(task_method);
		let onchain_task = TaskData {
			task:  SupportedTasks::SwapToken,
			task_data: format!("{}{}","this_is_the_chain_task", s).as_bytes().to_owned(),
			method: task_methods.clone(),
		};
		task.push(onchain_task);
		let mut task_data = Vec::new();
		let chain_tasks = OnchainTasks{
			task,
		};
		task_data.push(chain_tasks.clone());

	}: _(RawOrigin::Signed(who), chain.clone(), chain_tasks.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain.clone()), Some(task_data.clone()));
	}

	remove_chain_tasks {
		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;

	}: _(RawOrigin::Root, chain.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain.clone()), None);
	}

	impl_benchmark_test_suite!(OnChainTask, crate::mock::new_test_ext(), crate::mock::Test);
}