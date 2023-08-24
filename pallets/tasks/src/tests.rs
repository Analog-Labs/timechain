use crate::mock::*;
use crate::{
	Error, Event, NetworkShards, ShardTasks, TaskCycleState, TaskIdCounter, TaskPhaseState,
	TaskResults, TaskRetryCounter, TaskState, UnassignedTasks,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::Saturating;
use time_primitives::{
	CycleStatus, Function, Network, OcwTaskInterface, ShardId, TaskCycle, TaskDescriptor,
	TaskDescriptorParams, TaskError, TaskExecution, TaskPhase, TaskStatus, TasksInterface,
};

fn mock_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		function: Function::EvmViewCall {
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

fn mock_payable(network: Network) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		function: Function::EvmCall {
			address: Default::default(),
			function_signature: Default::default(),
			input: Default::default(),
			amount: 0,
		},
		cycle: 1,
		start: 0,
		period: 0,
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
		System::assert_last_event(Event::<Test>::TaskCreated(0).into());
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_eq!(
			Tasks::get_shard_tasks(1),
			vec![TaskExecution::new(0, 0, 0, TaskPhase::default())]
		);
		assert_ok!(Tasks::submit_task_result(0, 0, mock_result_ok(1)));
		System::assert_last_event(
			Event::<Test>::TaskResult(
				0,
				0,
				CycleStatus {
					shard_id: 1,
					signature: [0; 64],
				},
			)
			.into(),
		);
	});
}

#[test]
fn create_task_increments_task_id_counter() {
	new_test_ext().execute_with(|| {
		for i in 0..11 {
			assert_ok!(Tasks::create_task(
				RawOrigin::Signed([0; 32].into()).into(),
				mock_task(Network::Ethereum, 1)
			));
			assert_eq!(TaskIdCounter::<Test>::get(), i.saturating_plus_one());
		}
	});
}

#[test]
fn create_task_inserts_task_unassigned_sans_shards() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: [0; 32].into(),
				network: Network::Ethereum,
				function: Function::EvmViewCall {
					address: Default::default(),
					function_signature: Default::default(),
					input: Default::default(),
				},
				cycle: 1,
				start: 0,
				period: 1,
				hash: "".to_string(),
			}
		);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(
			UnassignedTasks::<Test>::iter().collect::<Vec<_>>(),
			vec![(Network::Ethereum, 0, ())]
		);
	});
}

#[test]
fn task_auto_assigned_if_shard_online() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: [0; 32].into(),
				network: Network::Ethereum,
				function: Function::EvmViewCall {
					address: Default::default(),
					function_signature: Default::default(),
					input: Default::default(),
				},
				cycle: 1,
				start: 0,
				period: 1,
				hash: "".to_string(),
			}
		);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>(), vec![]);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
	});
}

#[test]
fn task_auto_assigned_if_shard_joins_after() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: [0; 32].into(),
				network: Network::Ethereum,
				function: Function::EvmViewCall {
					address: Default::default(),
					function_signature: Default::default(),
					input: Default::default(),
				},
				cycle: 1,
				start: 0,
				period: 1,
				hash: "".to_string(),
			}
		);
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>(), vec![]);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
	});
}

#[test]
fn shard_online_inserts_network_shards() {
	new_test_ext().execute_with(|| {
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_none());
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_some());
	});
}

#[test]
fn shard_offline_removes_network_shards() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_some());
		assert_ok!(Tasks::shard_offline(1, Network::Ethereum));
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_none());
	});
}

#[test]
fn shard_offline_removes_tasks() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_ok!(Tasks::shard_offline(1, Network::Ethereum));
		assert_eq!(UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn shard_offline_assigns_tasks_if_other_shard_online() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(2, Network::Ethereum));
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(2, 0)]
		);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_ok!(Tasks::shard_offline(2, Network::Ethereum));
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(1, 0)]
		);
	});
}

#[test]
fn submit_completed_result_purges_task_from_storage() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::submit_task_result(0, 0, mock_result_ok(1)));
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn shard_offline_drops_failed_tasks() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		for _ in 0..4 {
			assert_ok!(Tasks::submit_task_error(
				0,
				TaskError {
					shard_id: 1,
					error: "test".to_string()
				}
			));
		}
		assert_ok!(Tasks::shard_offline(1, Network::Ethereum));
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn submit_task_error_increments_retry_count() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		let error = TaskError {
			shard_id: 1,
			error: "test".to_string(),
		};
		for _ in 1..=10 {
			assert_ok!(Tasks::submit_task_error(0, error.clone()));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
	});
}

#[test]
fn submit_task_error_over_max_retry_count_is_task_failure() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		let error = TaskError {
			shard_id: 1,
			error: "test".to_string(),
		};
		for _ in 0..4 {
			assert_ok!(Tasks::submit_task_error(0, error.clone()));
		}
		System::assert_last_event(Event::<Test>::TaskFailed(0, error).into());
	});
}

#[test]
fn submit_task_result_resets_retry_count() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		let error = TaskError {
			shard_id: 1,
			error: "test".to_string(),
		};
		for _ in 1..=10 {
			assert_ok!(Tasks::submit_task_error(0, error.clone()));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
		assert_ok!(Tasks::submit_task_result(0, 0, mock_result_ok(1)));
		assert_eq!(TaskRetryCounter::<Test>::get(0), 0);
	});
}

#[test]
fn test_cycle_must_be_greater_than_zero() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::create_task(
				RawOrigin::Signed([0; 32].into()).into(),
				mock_task(Network::Ethereum, 0)
			),
			Error::<Test>::CycleMustBeGreaterThanZero
		);
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
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Stopped));
		System::assert_last_event(Event::<Test>::TaskStopped(0).into());
	});
}

#[test]
fn cannot_stop_task_if_not_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_noop!(
			Tasks::stop_task(RawOrigin::Signed([1; 32].into()).into(), 0),
			Error::<Test>::InvalidOwner
		);
	});
}

#[test]
fn cannot_stop_stopped_task() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_noop!(
			Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0),
			Error::<Test>::InvalidTaskState
		);
	});
}

#[test]
fn cannot_stop_if_task_dne() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0),
			Error::<Test>::UnknownTask
		);
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
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Stopped));
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		System::assert_last_event(Event::<Test>::TaskResumed(0).into());
	});
}

#[test]
fn cannot_resume_if_task_dne() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0),
			Error::<Test>::UnknownTask
		);
	});
}

#[test]
fn cannot_resume_task_if_not_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_noop!(
			Tasks::resume_task(RawOrigin::Signed([1; 32].into()).into(), 0),
			Error::<Test>::InvalidOwner
		);
	});
}

#[test]
fn cannot_resume_running_task() {
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
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_ok!(Tasks::shard_offline(1, Network::Ethereum));
		assert_ok!(Tasks::shard_online(2, Network::Ethereum));
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Tasks::get_shard_tasks(1), vec![]);
		assert_eq!(
			Tasks::get_shard_tasks(2),
			vec![TaskExecution::new(0, 0, 0, TaskPhase::default())]
		);
	});
}

#[test]
fn task_recurring_cycle_count() {
	let mock_task = mock_task(Network::Ethereum, 5);
	let mut total_results = 0;
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), mock_task.clone()));
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		loop {
			let task = Tasks::get_shard_tasks(1);
			if task.is_empty() {
				break;
			}
			for task in task.iter() {
				let task_id = task.task_id;
				let cycle = task.cycle;
				assert_ok!(Tasks::submit_task_result(task_id, cycle, mock_result_ok(1)));
				total_results += 1;
			}
		}
		assert_eq!(total_results, mock_task.cycle);
	});
}

#[test]
fn schedule_tasks_assigns_tasks_to_least_assigned_shard() {
	new_test_ext().execute_with(|| {
		for i in (1..=10).rev() {
			assert_ok!(Tasks::shard_online(i, Network::Ethereum));
			for _ in 1..=i {
				assert_ok!(Tasks::create_task(
					RawOrigin::Signed([0; 32].into()).into(),
					mock_task(Network::Ethereum, 5)
				));
			}
		}
		for i in 1..=10 {
			assert_eq!(Tasks::get_shard_tasks(i).len() as u64, i);
		}
	});
}

#[test]
fn submit_task_result_inserts_at_input_cycle() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 5)
		));
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_ok!(Tasks::submit_task_result(0, 0, mock_result_ok(1)));
		assert_eq!(TaskCycleState::<Test>::get(0), 1);
		assert!(TaskResults::<Test>::get(0, 0).is_some());
		assert!(TaskResults::<Test>::get(0, 1).is_none());
		System::assert_last_event(Event::<Test>::TaskResult(0, 0, mock_result_ok(1)).into());
	});
}

#[test]
fn payable_task_smoke() {
	let shard_id = 1;
	let task_id = 0;
	let task_hash = "mock_hash";
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_payable(Network::Ethereum)
		));
		assert_ok!(Tasks::shard_online(1, Network::Ethereum));
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Write([0u8; 32]));
		assert_ok!(Tasks::submit_task_hash(shard_id, task_id, task_hash.into()));
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Read(Some(task_hash.into())));
		assert_ok!(Tasks::submit_task_result(
			task_id,
			0,
			CycleStatus { shard_id, signature: [0; 64] }
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
	});
}
