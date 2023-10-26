use crate::mock::*;
use crate::{
	Error, Event, NetworkShards, ShardTasks, TaskCycleState, TaskIdCounter, TaskPhaseState,
	TaskResults, TaskRetryCounter, TaskSignature, TaskState, UnassignedTasks,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::Saturating;
use time_primitives::{
	AccountId, Function, Network, PublicKey, ShardId, TaskCycle, TaskDescriptor,
	TaskDescriptorParams, TaskError, TaskExecution, TaskPhase, TaskResult, TaskStatus,
	TasksInterface,
};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

const A: [u8; 32] = [1u8; 32];

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

fn mock_sign_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		function: Function::SendMessage {
			contract_address: Default::default(),
			payload: Default::default(),
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

fn mock_result_ok(shard_id: ShardId) -> TaskResult {
	// these values are taken after running a valid instance of submitting result
	TaskResult {
		shard_id,
		hash: [
			11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91,
			0, 219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
		],
		signature: [
			196, 97, 193, 244, 167, 64, 220, 164, 96, 72, 191, 222, 230, 48, 188, 45, 36, 98, 135,
			148, 210, 252, 64, 5, 143, 222, 105, 229, 85, 45, 214, 253, 198, 248, 57, 75, 26, 186,
			131, 195, 9, 4, 74, 33, 247, 130, 248, 71, 140, 237, 153, 82, 248, 124, 78, 245, 18,
			185, 126, 32, 11, 210, 193, 48,
		],
	}
}

fn mock_error_result(shard_id: ShardId) -> TaskError {
	// these values are taken after running a valid instance of submitting error
	TaskError {
		shard_id,
		msg: "Invalid input length".into(),
		signature: [
			46, 103, 15, 236, 204, 108, 113, 94, 241, 4, 159, 152, 118, 95, 232, 215, 91, 198, 214,
			239, 85, 16, 143, 134, 114, 200, 38, 136, 205, 23, 145, 90, 178, 216, 139, 123, 140,
			95, 106, 243, 245, 5, 76, 210, 119, 221, 28, 253, 217, 79, 71, 255, 90, 216, 164, 157,
			68, 116, 211, 23, 223, 79, 30, 250,
		],
	}
}

#[test]
fn test_create_task() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		System::assert_last_event(Event::<Test>::TaskCreated(0).into());
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(
			Tasks::get_shard_tasks(1),
			vec![TaskExecution::new(0, 0, 0, TaskPhase::default())]
		);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1)
		));
		System::assert_last_event(Event::<Test>::TaskResult(0, 0, mock_result_ok(1)).into());
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
		Tasks::shard_online(1, Network::Ethereum);
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
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>(), vec![]);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
	});
}

#[test]
fn shard_online_inserts_network_shards() {
	new_test_ext().execute_with(|| {
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_none());
		Tasks::shard_online(1, Network::Ethereum);
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_some());
	});
}

#[test]
fn shard_offline_removes_network_shards() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_some());
		Tasks::shard_offline(1, Network::Ethereum);
		assert!(NetworkShards::<Test>::get(Network::Ethereum, 1).is_none());
	});
}

#[test]
fn shard_offline_removes_tasks() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		Tasks::shard_offline(1, Network::Ethereum);
		assert_eq!(UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn shard_offline_assigns_tasks_if_other_shard_online() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(2, Network::Ethereum);
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(2, 0)]
		);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		Tasks::shard_offline(2, Network::Ethereum);
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
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1)
		));
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn shard_offline_doesnt_drops_failed_tasks() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		for _ in 0..4 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error_result(1)
			));
		}
		Tasks::shard_offline(1, Network::Ethereum);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len() == 1);
	});
}

#[test]
fn submit_task_error_increments_retry_count() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		for _ in 1..=10 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error_result(1)
			));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
	});
}

#[test]
fn submit_task_error_over_max_retry_count_is_task_failure() {
	new_test_ext().execute_with(|| {
		let error = mock_error_result(1);
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		for _ in 1..4 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				error.clone()
			));
		}
		System::assert_last_event(Event::<Test>::TaskFailed(0, 0, error).into());
	});
}

#[test]
fn submit_task_result_resets_retry_count() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		for _ in 1..=10 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error_result(1)
			));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1)
		));
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
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		System::assert_last_event(Event::<Test>::TaskResumed(0).into());
	});
}

#[test]
fn task_resumed_by_root() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Root.into(), 0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		assert_ok!(Tasks::resume_task(RawOrigin::Root.into(), 0, 0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		System::assert_last_event(Event::<Test>::TaskResumed(0).into());
	});
}

#[test]
fn task_stopped_by_invalid_owner() {
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
fn cannot_resume_if_task_dne() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0),
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
			Tasks::resume_task(RawOrigin::Signed([1; 32].into()).into(), 0, 0),
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
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0),
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
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0));
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
		Tasks::shard_online(1, Network::Ethereum);
		loop {
			let task = Tasks::get_shard_tasks(1);
			if task.is_empty() {
				break;
			}
			for task in &task {
				let task_id = task.task_id;
				let cycle = task.cycle;
				assert_ok!(Tasks::submit_result(
					RawOrigin::Signed([0; 32].into()).into(),
					task_id,
					cycle,
					mock_result_ok(1)
				));
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
			Tasks::shard_online(i, Network::Ethereum);
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
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1)
		));
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
	let task_cycle = 0;
	let task_hash = "mock_hash";
	let a: AccountId = A.into();
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed(a.clone()).into(),
			mock_payable(Network::Ethereum)
		));
		Tasks::shard_online(shard_id, Network::Ethereum);
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Write(pubkey_from_bytes(A)));
		assert_ok!(Tasks::submit_hash(
			RawOrigin::Signed(a.clone()).into(),
			task_id,
			task_cycle,
			task_hash.into()
		));
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Read(Some(task_hash.into())));
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed(a).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
	});
}

#[test]
fn resume_failed_task_after_shard_offline() {
	let mock_error = mock_error_result(1);
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		Tasks::shard_online(1, Network::Ethereum);
		// fails 3 time to turn task status to failed
		for _ in 0..3 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error.clone()
			));
		}
		assert_eq!(Tasks::task_shard(0), Some(1));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Failed { error: mock_error }));
		Tasks::shard_offline(1, Network::Ethereum);
		assert_eq!(Tasks::task_shard(0), None);
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0));
		assert_eq!(Tasks::task_shard(0), Some(1));
	});
}

#[test]
fn submit_signature_inserts_signature_into_storage() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),);
		assert_eq!(TaskSignature::<Test>::get(0), Some([0u8; 64]));
	});
}

#[test]
fn submit_signature_fails_when_task_dne() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),
			Error::<Test>::UnknownTask
		);
	});
}

#[test]
fn submit_signature_fails_if_not_sign_phase() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),
			Error::<Test>::NotSignPhase
		);
	});
}

#[test]
fn submit_signature_fails_if_unassigned() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),
			Error::<Test>::UnassignedTask
		);
	});
}

#[test]
fn submit_signature_fails_after_called_once() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),);
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),
			Error::<Test>::TaskSigned
		);
	});
}
