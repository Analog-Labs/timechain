use super::*;

#[allow(unused)]
use crate::Pallet as OnChainTask;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_std::borrow::ToOwned;
use crate::{types::*};
use sp_std::prelude::*;

benchmarks! {


	store_onchain_task {

		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;
		let mut task = Vec::new();
		let mut task_methods = Vec::new();
		let task_method = TaskMethod {
			name: "this_is_the_chain_method".as_bytes().to_owned(),
			arguments: "this_is_the_chain_arguments".as_bytes().to_owned(),
		};
		task_methods.push(task_method);
		let onchain_task = TaskData {
			task:  ChainTask::SwapToken,
			chain_data: "this_is_the_chain_data".as_bytes().to_owned(),
			method: task_methods.clone(),
		};
		task.push(onchain_task);
		let mut chain_task = Vec::new();
		let chain_tasks = 	OnchainTaskData{
			task,
		};
		chain_task.push(chain_tasks.clone());

	}: _(RawOrigin::Signed(who), chain.clone(), chain_tasks.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain.clone()), Some(chain_task.clone()));
	}


	remove_onchain_task {
		let who: T::AccountId = whitelisted_caller();
		let chain: SupportedChain = SupportedChain::Timechain;

	}: _(RawOrigin::Root, chain.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain.clone()), None);
	}

	impl_benchmark_test_suite!(OnChainTask, crate::mock::new_test_ext(), crate::mock::Test);
}