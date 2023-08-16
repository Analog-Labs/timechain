use crate::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{
	CycleStatus, Function, Network, OcwSubmitTaskResult, ScheduleInterface, ShardId, TaskCycle,
	TaskDescriptorParams, TaskExecution,
};

fn mock_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		function: Function::EVMViewWithoutAbi {
			address: Default::default(),
			function_signature: Default::default(),
			input: Default::default(),
		},
		cycle,
		start: 0,
		period: 1,
		hash: "".to_string(),
	}
}

fn mock_result_ok(shard_id: ShardId) -> CycleStatus {
	CycleStatus { shard_id, signature: [0; 64] }
}

#[test]
fn test_create_task() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(Tasks::get_shard_tasks(1), vec![TaskExecution::new(0, 0, 0)]);
		assert_ok!(Tasks::submit_task_result(0, 0, mock_result_ok(1)));
	});
}
