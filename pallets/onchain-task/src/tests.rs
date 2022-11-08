use super::mock::*;
use crate::types::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn adding_tesseract_task() {
	new_test_ext().execute_with(|| {
		// Call the tesseract add task extrinsic
		assert_ok!(OnChainTask::add_task(RawOrigin::Root.into(), 1, TesseractTask::AddChain));

		// Retreiving the task that has been stored.
		assert_eq!(OnChainTask::tesseract_tasks(1), Some(TesseractTask::AddChain));
	});
}

#[test]
fn removing_tesseract_task() {
	new_test_ext().execute_with(|| {
		// Call the tesseract add task extrinsic
		assert_ok!(OnChainTask::add_task( RawOrigin::Root.into(), 1, TesseractTask::AddChain));

		// Retreiving the task that has been stored.
		assert_eq!(OnChainTask::tesseract_tasks(1), Some(TesseractTask::AddChain));

		// Call the tesseract remove task extrinsic
		assert_ok!(OnChainTask::remove_task(RawOrigin::Root.into(), 1,));
		// Checking that the task has been rmoved
		assert_eq!(OnChainTask::tesseract_tasks(1), None);
	});
}

#[test]
fn storing_and_get_chain_key() {
	let chain_data = "this_is_the_chain_data".as_bytes().to_owned();
	let chain_methods = "this_is_the_chain_method".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// We first add the Task with root privilege
		assert_ok!(OnChainTask::add_task(RawOrigin::Root.into(), 1, TesseractTask::AddChain));
		//let account = AccountKeyring::Alice.to_account_id().;
		let random_hash = OnChainTask::random_hash(&1);
		let stored_chain_data = OnchainTaskData{
			id: random_hash.clone(),
			chain_data: chain_data.clone(),
			methods: chain_methods.clone(),
		};

		// Call the store task signature extrinsic
		assert_ok!(OnChainTask::store_onchain_task(
			RawOrigin::Signed(1).into(),
			stored_chain_data.clone(),
		));
		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(OnChainTask::task_store(stored_chain_data.id), Some(stored_chain_data));
	});
}