use super::mock::*;
use crate::types::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;


#[test]
fn storing_and_get_chain_data() {
	let chain: SupportedChain = SupportedChain::Timechain;
	let new_task: ChainTask = ChainTask::SwapToken;
	let chain_data: ChainData = "this_is_the_chain_data".as_bytes().to_owned();
	let chain_methods: MethodName = "this_is_the_chain_method".as_bytes().to_owned();
	let chain_arguments: MethodArguments = "this_is_the_chain_arguments".as_bytes().to_owned();
	let mut task = Vec::new();
	let mut task_methods = Vec::new();
	let task_method = TaskMethod {
		name: chain_methods,
		arguments: chain_arguments,
	};
	task_methods.push(task_method);
	let onchain_task = TaskData {
		task: new_task.clone(),
		chain_data: chain_data.clone(),
		method: task_methods.clone(),
	};
	task.push(onchain_task);
	let mut chain_task = Vec::new();
	let chain_tasks = 	OnchainTaskData{
		task,
	};
	chain_task.push(chain_tasks.clone());

	new_test_ext().execute_with(|| {
		// Call the store task  extrinsic
		assert_ok!(OnChainTask::store_onchain_task(
			RawOrigin::Signed(1).into(),
			chain.clone(),
			chain_tasks.clone(),

		));
		// Retreiving the task stored via it's key and assert the result.
		assert_eq!(OnChainTask::task_store(chain.clone()), Some(chain_task.clone()));
	});
}

#[test]
fn edit_chain_task_data() {
	let chain: SupportedChain = SupportedChain::Timechain;
	let mut task = Vec::new();
	let mut task2 = Vec::new();
	let mut task_methods = Vec::new();
	let task_method = TaskMethod {
		name: "this_is_the_chain_method".as_bytes().to_owned(),
		arguments: "this_is_the_chain_arguments".as_bytes().to_owned(),
	};
	task_methods.push(task_method);
	let onchain_task = TaskData {
		task: ChainTask::SwapToken,
		chain_data: "this_is_the_chain_data".as_bytes().to_owned(),
		method: task_methods.clone(),
	};
	let onchain_task2 = TaskData {
		task: ChainTask::SwapToken,
		chain_data: "this_is_the_chain_data2".as_bytes().to_owned(),
		method: task_methods.clone(),
	};
	task.push(onchain_task);
	task2.push(onchain_task2);
	let mut chain_task = Vec::new();
	let mut chain_task2 = Vec::new();
	let chain_tasks = 	OnchainTaskData{
		task,
	};
	let chain_tasks2 = OnchainTaskData{
		task: task2,
	};
	chain_task.push(chain_tasks.clone());
	chain_task2.push(chain_tasks2.clone());

	new_test_ext().execute_with(|| {
		// Call the store task  extrinsic
		assert_ok!(OnChainTask::edit_task_data(
			RawOrigin::Signed(1).into(),
			chain.clone(),
			chain_tasks.clone(),
			chain_tasks2.clone(),

		));
		// Retreiving the task stored via it's key and assert the result.
		assert_eq!(OnChainTask::task_store(chain.clone()), Some(chain_task2.clone()));
	});
}

#[test]
fn it_works_removing_onchain_task() {

	let chain: SupportedChain = SupportedChain::Timechain;

	new_test_ext().execute_with(|| {
		// Call the remove task extrinsic
		assert_ok!(OnChainTask::remove_onchain_task(RawOrigin::Root.into(), chain.clone()));
		// Checking that the task has been rmoved
		assert_eq!(OnChainTask::task_store(chain.clone()), None);
	});
}
