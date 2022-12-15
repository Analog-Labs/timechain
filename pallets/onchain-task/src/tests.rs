use super::mock::*;
use crate::types::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn storing_and_get_chain_task() {
	let chain: SupportedChain = SupportedChain::Timechain;
	let task_metadata = OnChainTaskMetadata {
		task: SupportedTasks::EthereumTasks(EthereumTasks::SwapToken),
		arguments: vec![vec![]],
	};

	let frequency = 100;

	new_test_ext().execute_with(|| {
		let task_id = OnChainTask::next_task_id();
		assert_eq!(task_id, None);

		// Call the store task  extrinsic
		assert_ok!(OnChainTask::store_task(
			RawOrigin::Signed(1).into(),
			chain,
			task_metadata.clone(),
			frequency,
		));

		// check task id updated.
		let task_id = OnChainTask::next_task_id();
		assert_eq!(task_id, Some(0));

		let onchain_task = OnchainTask { task_id: 0, frequency };

		// check added task
		let tasks = OnChainTask::task_store(chain);
		assert_eq!(tasks.clone().unwrap().len(), 1);
		assert_eq!(tasks.unwrap()[0], onchain_task);

		// check metadata
		let stored_metadata = OnChainTask::task_metadata(0).unwrap();
		assert_eq!(stored_metadata, task_metadata.clone());

		// check metadata index
		let metadata_index = OnChainTask::task_metadata_id(task_metadata).unwrap();
		assert_eq!(0, metadata_index);
	});
}
