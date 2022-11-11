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
		let new_task: ChainTask = ChainTask::SwapToken;
		let chain_id: ChainId = "this_is_the_chain_id".as_bytes().to_owned();
		let chain_data: ChainData = "this_is_the_chain_data".as_bytes().to_owned();
		let chain_methods: MethodName = "this_is_the_chain_method".as_bytes().to_owned();
		let chain_arguments: MethodArguments = "this_is_the_chain_arguments".as_bytes().to_owned();
		let mut task = Vec::new();
		let task_method = TaskMethod {
			name: chain_methods,
			arguments: chain_arguments,
		};
		let onchain_task = OnchainTaskData {
			task: new_task.clone(),
			chain_data: chain_data.clone(),
			method: task_method.clone(),
		};
		task.push(onchain_task);


	}: _(RawOrigin::Signed(who), chain.clone(), new_task.clone(), chain_id.clone(), chain_data.clone(), task_method.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain_id.clone(), chain.clone()), Some(task.clone()));
	}


	remove_onchain_task {
		let who: T::AccountId = whitelisted_caller();

		let chain: SupportedChain = SupportedChain::Timechain;
		let new_task: ChainTask = ChainTask::SwapToken;
		let chain_id: ChainId = "this_is_the_chain_id".as_bytes().to_owned();
		let chain_data: ChainData = "this_is_the_chain_data".as_bytes().to_owned();
		let chain_methods: MethodName = "this_is_the_chain_method".as_bytes().to_owned();
		let chain_arguments: MethodArguments = "this_is_the_chain_arguments".as_bytes().to_owned();
		let mut task = Vec::new();
		let task_methods = TaskMethod {
			name: chain_methods,
			arguments: chain_arguments,
		};
		let onchain_task = OnchainTaskData {
			task: new_task.clone(),
			chain_data: chain_data.clone(),
			method: task_methods.clone(),
		};
		task.push(onchain_task);

	}: _(RawOrigin::Root, chain.clone(), chain_data.clone())
	verify {
		assert_eq!(OnchainTaskStore::<T>::get(chain_id.clone(), chain.clone()), None);
	}

	impl_benchmark_test_suite!(OnChainTask, crate::mock::new_test_ext(), crate::mock::Test);
}