use crate::mock::*;
use crate::{Error, Event};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::{
	CycleStatus, Function, Network, OcwSubmitTaskResult, ScheduleInterface, ShardId, TaskCycle,
	TaskDescriptorParams, TaskExecution, TaskStatus, TaskError
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

#[test]
fn task_stopped_by_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		System::assert_last_event(Event::<Test>::TaskStopped(0).into());
	});
}

#[test]
fn task_resumed_by_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		System::assert_last_event(Event::<Test>::TaskResumed(0).into());
	});
}

#[test]
fn resume_failed_task() {
	let mock_error = TaskError {
					shard_id: 1,
					error: "mock_error".to_string()
				};
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		// fails 3 time to turn task status to failed
		for _ in 0..3 {
			assert_ok!(Tasks::submit_task_error(
				0,
				mock_error.clone()
			));
		}
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Failed { error: mock_error }));
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0),);
	});
}

#[test]
fn task_invalid_task_state_during_resume() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_noop!(
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0),
			Error::<Test>::InvalidTaskState
		);
	});
}

#[test]
fn task_stopped_and_moved_on_shard_offline() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		Tasks::shard_offline(1, Network::Ethereum);
		Tasks::shard_online(2, Network::Ethereum);
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Tasks::get_shard_tasks(1), vec![]);
		assert_eq!(Tasks::get_shard_tasks(2), vec![TaskExecution::new(0, 0, 0)]);
	});
}

#[test]
fn task_recurring_cycle_count() {
	let mock_task = mock_task(Network::Ethereum, 5);
	let mut total_results = 0;
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), mock_task.clone()));
		Tasks::shard_online(1, Network::Ethereum);
		loop {
			let task = Tasks::get_shard_tasks(1);
			if task.is_empty() {
				break;
			}
			for task in task.iter().copied() {
				let task_id = task.task_id;
				let cycle = task.cycle;
				assert_ok!(Tasks::submit_task_result(task_id, cycle, mock_result_ok(1)));
				total_results += 1;
			}
		}
		assert_eq!(total_results, mock_task.cycle);
	});
}
