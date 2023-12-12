use crate::mock::*;
use crate::{
	Error, Event, Gateway, NetworkShards, ShardRegistered, ShardTasks, TaskCycleState,
	TaskIdCounter, TaskPhaseState, TaskResults, TaskRetryCounter, TaskSignature, TaskState,
	UnassignedTasks,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use schnorr_evm::VerifyingKey;
use sp_runtime::Saturating;
use time_primitives::{
	append_hash_with_task_data, AccountId, Function, Network, PublicKey, ShardId, TaskCycle,
	TaskDescriptor, TaskDescriptorParams, TaskError, TaskExecution, TaskId, TaskPhase, TaskResult,
	TaskStatus, TasksInterface,
};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

const A: [u8; 32] = [1u8; 32];

fn mock_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		cycle,
		start: 0,
		period: 1,
		timegraph: None,
		function: Function::EvmViewCall {
			address: Default::default(),
			input: Default::default(),
		},
	}
}

fn mock_sign_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		cycle,
		start: 0,
		period: 1,
		timegraph: None,
		function: Function::SendMessage {
			address: Default::default(),
			gas_limit: Default::default(),
			salt: Default::default(),
			payload: Default::default(),
		},
	}
}

fn mock_payable(network: Network) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		cycle: 1,
		start: 0,
		period: 0,
		timegraph: None,
		function: Function::EvmCall {
			address: Default::default(),
			input: Default::default(),
			amount: 0,
		},
	}
}

fn mock_result_ok(shard_id: ShardId, task_id: TaskId, task_cycle: TaskCycle) -> TaskResult {
	// these values are taken after running a valid instance of submitting result
	let hash = [
		11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91, 0,
		219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
	];
	let appended_hash = append_hash_with_task_data(hash, task_id, task_cycle);
	let final_hash = VerifyingKey::message_hash(&appended_hash);
	let signature = MockTssSigner::new().sign(final_hash).to_bytes();
	TaskResult { shard_id, hash, signature }
}

fn mock_error_result(shard_id: ShardId, task_id: TaskId, task_cycle: TaskCycle) -> TaskError {
	// these values are taken after running a valid instance of submitting error
	let msg: String = "Invalid input length".into();
	let msg_hash = VerifyingKey::message_hash(msg.as_bytes());
	let hash = append_hash_with_task_data(msg_hash, task_id, task_cycle);
	let final_hash = VerifyingKey::message_hash(&hash);
	let signature = MockTssSigner::new().sign(final_hash).to_bytes();
	TaskError { shard_id, msg, signature }
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
		System::assert_last_event(Event::<Test>::TaskCreated(1).into());
		assert_eq!(
			Tasks::get_shard_tasks(1),
			vec![
				TaskExecution {
					task_id: 1,
					cycle: 0,
					retry_count: 0,
					phase: TaskPhase::Sign
				},
				TaskExecution::new(0, 0, 0, TaskPhase::default()),
			]
		);
		let task_result = mock_result_ok(1, 0, 0);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			task_result.clone()
		));
		System::assert_last_event(Event::<Test>::TaskResult(0, 0, task_result).into());
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
				owner: Some([0; 32].into()),
				network: Network::Ethereum,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
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
			Tasks::tasks(1).unwrap(),
			TaskDescriptor {
				owner: Some([0; 32].into()),
				network: Network::Ethereum,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
			}
		);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>(), vec![]);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1, 0]);
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
				owner: Some([0; 32].into()),
				network: Network::Ethereum,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
			}
		);
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>(), vec![]);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1, 0]);
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
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1, 0]);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		// complete task to register shard
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1, 0, 0)
		));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1, 0]);
		Tasks::shard_offline(1, Network::Ethereum);
		assert_eq!(
			UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(),
			vec![1, 0, 2]
		);
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
			vec![(1, 1), (2, 0), (2, 2)]
		);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		Tasks::shard_offline(2, Network::Ethereum);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(1, 3), (1, 1), (1, 0), (1, 2)]
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
			mock_result_ok(1, 0, 0)
		));
		assert_eq!(ShardTasks::<Test>::iter().collect::<Vec<_>>().len(), 1);
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
				mock_error_result(1, 0, 0)
			));
		}
		Tasks::shard_offline(1, Network::Ethereum);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 3);
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
				mock_error_result(1, 0, 0)
			));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
	});
}

#[test]
fn submit_task_error_over_max_retry_count_is_task_failure() {
	new_test_ext().execute_with(|| {
		let error = mock_error_result(1, 0, 0);
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
				mock_error_result(1, 0, 0)
			));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1, 0, 0)
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
			vec![
				TaskExecution {
					task_id: 3,
					cycle: 0,
					retry_count: 0,
					phase: TaskPhase::Sign
				},
				TaskExecution {
					task_id: 1,
					cycle: 0,
					retry_count: 0,
					phase: TaskPhase::Sign
				},
				TaskExecution::new(0, 0, 0, TaskPhase::default()),
				TaskExecution {
					task_id: 2,
					cycle: 0,
					retry_count: 0,
					phase: TaskPhase::Sign
				}
			]
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
			let tasks = Tasks::get_shard_tasks(1);
			if tasks.len() == 1 {
				break;
			}
			for task in &tasks {
				if task.phase != TaskPhase::Sign {
					let task_id = task.task_id;
					let cycle = task.cycle;
					assert_ok!(Tasks::submit_result(
						RawOrigin::Signed([0; 32].into()).into(),
						task_id,
						cycle,
						mock_result_ok(1, task_id, cycle)
					));
					total_results += 1;
				}
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
			assert_eq!(Tasks::get_shard_tasks(i).len() as u64, i + 1);
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
		let task_result = mock_result_ok(1, 0, 0);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			task_result.clone()
		));
		assert_eq!(TaskCycleState::<Test>::get(0), 1);
		assert!(TaskResults::<Test>::get(0, 0).is_some());
		assert!(TaskResults::<Test>::get(0, 1).is_none());
		System::assert_last_event(Event::<Test>::TaskResult(0, 0, task_result).into());
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
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
	});
}

#[test]
fn resume_failed_task_after_shard_offline() {
	let mock_error = mock_error_result(1, 0, 0);
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

#[test]
fn register_gateway_fails_if_not_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::register_gateway(
				RawOrigin::Signed([0; 32].into()).into(),
				1,
				[0u8; 20].to_vec(),
			),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn register_gateway_fails_if_unknown_shard() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),),
			Error::<Test>::UnknownShard
		);
	});
}

#[test]
fn register_gateway_emits_event() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),),);
		System::assert_last_event(
			Event::<Test>::GatewayRegistered(Network::Ethereum, [0u8; 20].to_vec()).into(),
		);
	});
}

#[test]
fn register_gateway_updates_shard_registered_storage() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),),);
		assert_eq!(ShardRegistered::<Test>::get(1), Some(()));
	});
}

#[test]
fn register_gateway_updates_gateway_storage() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),),);
		assert_eq!(Gateway::<Test>::get(Network::Ethereum), Some([0u8; 20].to_vec()));
	});
}

#[test]
fn shard_online_starts_register_shard_task() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: None,
				network: Network::Ethereum,
				function: Function::RegisterShard { shard_id: 1 },
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
			}
		);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		// complete task to register shard
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1, 0, 0)
		));
		//when a register shard task is complete the shard is marked as registered
		assert_eq!(ShardRegistered::<Test>::get(1), Some(()));
	});
}

#[test]
fn register_gateway_completes_register_shard_task() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),),);
		assert_eq!(ShardRegistered::<Test>::get(1), Some(()));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Completed));
		// submit result still succeeds because task is completed
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1, 0, 0)
		));
	});
}

#[test]
fn shard_offline_starts_unregister_shard_task() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		// complete task to register shard
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(1, 0, 0)
		));
		assert_eq!(ShardRegistered::<Test>::get(1), Some(()));
		Tasks::shard_offline(1, Network::Ethereum);
		assert_eq!(
			Tasks::tasks(1).unwrap(),
			TaskDescriptor {
				owner: None,
				network: Network::Ethereum,
				function: Function::UnregisterShard { shard_id: 1 },
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
			}
		);
		assert_eq!(Tasks::task_state(1), Some(TaskStatus::Created));
		// complete task to unregister shard
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			1,
			0,
			mock_result_ok(0, 1, 0)
		));
		assert_eq!(ShardRegistered::<Test>::get(1), None);
	});
}

#[test]
fn shard_offline_stops_pending_register_shard_task() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		Tasks::shard_offline(1, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		// shard not registered
		assert_eq!(ShardRegistered::<Test>::get(1), None);
		// task to register shard fails because it was stopped by `shard_offline`
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_result_ok(1, 0, 0)
			),
			Error::<Test>::TaskStoppedOrFailed
		);
	});
}

#[test]
fn shard_offline_does_not_schedule_unregister_if_shard_not_registered() {
	new_test_ext().execute_with(|| {
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		Tasks::shard_offline(1, Network::Ethereum);
		assert!(Tasks::tasks(1).is_none());
		assert_eq!(Tasks::task_state(1), None);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		// task to unregister shard does not exist
		assert!(Tasks::tasks(1).is_none());
		assert_eq!(Tasks::task_state(1), None);
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0; 32].into()).into(),
				1,
				0,
				mock_result_ok(1, 1, 0)
			),
			Error::<Test>::UnknownTask
		);
	});
}
