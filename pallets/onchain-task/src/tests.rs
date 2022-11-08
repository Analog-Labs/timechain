use super::mock::*;
use crate::types::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn adding_tesseract_task() {
	new_test_ext().execute_with(|| {
		// Call the tesseract add task extrinsic
		assert_ok!(OnChainTask::add_task(RawOrigin::Signed(1).into(), 1, TesseractTask::AddChain));

		// Retreiving the task that has been stored.
		assert_eq!(OnChainTask::tesseract_tasks(1), Some(TesseractTask::AddChain));
	});
}

#[test]
fn removing_tesseract_task() {
	new_test_ext().execute_with(|| {
		// Call the tesseract add task extrinsic
		assert_ok!(OnChainTask::add_task( RawOrigin::Signed(1).into(), 1, TesseractTask::AddChain));

		// Retreiving the task that has been stored.
		assert_eq!(OnChainTask::tesseract_tasks(1), Some(TesseractTask::AddChain));

		// Call the tesseract remove task extrinsic
		assert_ok!(OnChainTask::remove_task(RawOrigin::Signed(1).into(), 1,));
		// Checking that the task has been rmoved
		assert_eq!(OnChainTask::tesseract_tasks(1), None);
	});
}

#[test]
fn storing_and_get_chain_key() {
	let chain_data: ChainData = "this_is_the_chain_data".as_bytes().to_owned();
	let chain_methods: Methods = "this_is_the_chain_method".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// We first add the Task with root privilege
		assert_ok!(OnChainTask::add_task(RawOrigin::Signed(1).into(), 1, TesseractTask::AddChain));
		let account_id: u64 = 0;
		// Call the store task signature extrinsic
		assert_ok!(OnChainTask::store_onchain_task(
			RawOrigin::Signed(1).into(),
			chain_data.clone(),
			chain_methods.clone(),
		));
		// Retreiving the signature stored via it's key and assert the result.
		// assert_eq!(OnChainTask::task_store(account_id), Some(chain_methods.clone()));
	});
}