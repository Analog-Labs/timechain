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
		// Call the store task  extrinsic
		assert_ok!(OnChainTask::store_task(
			RawOrigin::Signed(1).into(),
			chain,
			task_metadata,
			frequency,
		));
	});
}
