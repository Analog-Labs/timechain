use super::mock::*;
use crate::types::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;


#[test]
fn storing_and_get_chain_data() {
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

	new_test_ext().execute_with(|| {
		// Call the store task  extrinsic
		assert_ok!(OnChainTask::store_onchain_task(
			RawOrigin::Signed(1).into(),
			chain.clone(),
			new_task.clone(),
			chain_id.clone(),
			chain_data.clone(),
			task_methods.clone(),

		));
		// Retreiving the task stored via it's key and assert the result.
		assert_eq!(OnChainTask::task_store(chain_id.clone(), chain.clone()), Some(task.clone()));
	});
}

#[test]
fn it_works_removing_tesseract_member() {

	let chain: SupportedChain = SupportedChain::Timechain;
	let chain_id: ChainId = "this_is_the_chain_id".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// Call the remove task extrinsic
		assert_ok!(OnChainTask::remove_onchain_task(RawOrigin::Root.into(), chain.clone(), chain_id.clone()));
		// Checking that the task has been rmoved
		assert_eq!(OnChainTask::task_store(chain_id.clone(), chain.clone()), None);
	});
}
