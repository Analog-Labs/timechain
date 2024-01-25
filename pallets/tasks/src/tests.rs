use crate::mock::*;
use crate::{
	Error, Event, Gateway, MemberPayout, NetworkReadReward, NetworkSendMessageReward,
	NetworkShards, NetworkWriteReward, ShardRegistered, ShardTasks, SignerPayout, TaskCycleState,
	TaskIdCounter, TaskPhaseState, TaskResults, TaskRetryCounter, TaskRewardConfig, TaskSignature,
	TaskState, TotalPayout, UnassignedTasks,
};
use frame_support::traits::Get;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use schnorr_evm::VerifyingKey;
use sp_runtime::Saturating;
use time_primitives::{
	append_hash_with_task_data, AccountId, Function, Network, PublicKey, RewardConfig, ShardId,
	ShardStatus, ShardsInterface, TaskCycle, TaskDescriptor, TaskDescriptorParams, TaskError,
	TaskExecution, TaskId, TaskPhase, TaskResult, TaskStatus, TasksInterface,
};

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

const A: [u8; 32] = [1u8; 32];

fn mock_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams<u128> {
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
		funds: 100,
		shard_size: 3,
	}
}

fn mock_sign_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams<u128> {
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
		funds: 100,
		shard_size: 3,
	}
}

fn mock_payable(network: Network) -> TaskDescriptorParams<u128> {
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
		funds: 100,
		shard_size: 3,
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
	let msg_hash = VerifyingKey::message_hash(&msg.clone().into_bytes());
	let hash = append_hash_with_task_data(msg_hash, task_id, task_cycle);
	let final_hash = VerifyingKey::message_hash(&hash);
	let signature = MockTssSigner::new().sign(final_hash).to_bytes();
	TaskError { shard_id, msg, signature }
}

#[test]
fn test_create_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		System::assert_last_event(Event::<Test>::TaskCreated(0).into());
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		System::assert_last_event(Event::<Test>::TaskCreated(1).into());
		assert_eq!(
			Tasks::get_shard_tasks(0),
			vec![TaskExecution::new(0, 0, 0, TaskPhase::default()),]
		);
		let mut read_task_reward: u128 = <Test as crate::Config>::BaseReadReward::get();
		read_task_reward =
			read_task_reward.saturating_add(NetworkReadReward::<Test>::get(Network::Ethereum));
		let mut write_task_reward: u128 = <Test as crate::Config>::BaseWriteReward::get();
		write_task_reward =
			write_task_reward.saturating_add(NetworkWriteReward::<Test>::get(Network::Ethereum));
		let mut send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		send_message_reward = send_message_reward
			.saturating_add(NetworkSendMessageReward::<Test>::get(Network::Ethereum));
		assert_eq!(
			TaskRewardConfig::<Test>::get(0),
			Some(RewardConfig {
				read_task_reward,
				write_task_reward,
				send_message_reward,
				depreciation_rate: <Test as crate::Config>::RewardDeclineRate::get(),
			})
		);
		assert_eq!(Tasks::tasks(0).unwrap().shard_size, 3);
		// insert shard public key to match mock result signature
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let task_result = mock_result_ok(0, 0, 0);
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
				shard_size: 3,
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
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
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
				shard_size: 3,
			}
		);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 1);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1]);
	});
}

#[test]
fn task_auto_assigned_if_shard_joins_after() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
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
				shard_size: 3,
			}
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 1);
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
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1]);
		assert_eq!(UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1, 0]);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, Network::Ethereum);
		assert_eq!(
			UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(),
			vec![1, 2]
		);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn shard_offline_assigns_tasks_if_other_shard_online() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(0, 2)]
		);
		assert_eq!(
			UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(),
			vec![1, 0]
		);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, Network::Ethereum);
		assert_eq!(
			UnassignedTasks::<Test>::iter().collect::<Vec<_>>(),
			vec![(Network::Ethereum, 1, ()), (Network::Ethereum, 0, ())]
		);
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(1, 2)]
		);
	});
}

#[test]
fn submit_completed_result_purges_task_from_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(0, 0, 0)
		));
		assert_eq!(ShardTasks::<Test>::iter().collect::<Vec<_>>().len(), 2);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

// TODO: is this change in behavior intended? intended to not drop failed tasks but test was incorrect before
#[test]
fn shard_offline_drops_failed_tasks() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		for _ in 0..4 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error_result(0, 0, 0)
			));
		}
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_offline(0, Network::Ethereum);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 2);
	});
}

#[test]
fn submit_task_error_increments_retry_count() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		for _ in 1..=10 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error_result(0, 0, 0)
			));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
	});
}

#[test]
fn submit_task_error_over_max_retry_count_is_task_failure() {
	new_test_ext().execute_with(|| {
		let error = mock_error_result(0, 0, 0);
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
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
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		for _ in 1..=10 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error_result(0, 0, 0)
			));
		}
		assert_eq!(TaskRetryCounter::<Test>::get(0), 10);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			mock_result_ok(0, 0, 0)
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
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 10));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		System::assert_last_event(Event::<Test>::TaskResumed(0).into());
	});
}

#[test]
fn root_may_resume_unfunded_task() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Root.into(), 0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		assert_ok!(Tasks::resume_task(RawOrigin::Root.into(), 0, 0, 10));
	});
}

#[test]
fn task_resumable_by_root() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		// Task is stopped but not drained as expected so may be resumed by root
		TaskState::<Test>::insert(0, TaskStatus::Stopped);
		assert_ok!(Tasks::resume_task(RawOrigin::Root.into(), 0, 0, 10));
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
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 0),
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
			Tasks::resume_task(RawOrigin::Signed([1; 32].into()).into(), 0, 0, 10),
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
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 0),
			Error::<Test>::InvalidTaskState
		);
	});
}

#[test]
fn task_stopped_and_moved_on_shard_offline() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Shards::create_shard(
			Network::Ethereum,
			[[1u8; 32].into(), [2u8; 32].into(), [3u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(
			Tasks::get_shard_tasks(0),
			vec![TaskExecution::new(0, 0, 0, TaskPhase::default()),]
		);
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Tasks::get_shard_tasks(0), vec![]);
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 10));
		Tasks::shard_offline(0, Network::Ethereum);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, Network::Ethereum);
		assert_eq!(Tasks::get_shard_tasks(0), vec![]);
		assert_eq!(
			Tasks::get_shard_tasks(1),
			vec![TaskExecution::new(0, 0, 0, TaskPhase::default()),]
		);
	});
}

#[test]
fn task_recurring_cycle_count() {
	let mock_task = mock_task(Network::Ethereum, 5);
	let mut total_results = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), mock_task.clone()));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		let tasks = Tasks::get_shard_tasks(0);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		for task in &tasks {
			let task_id = task.task_id;
			let mut cycle = task.cycle;
			for _ in 0..mock_task.cycle {
				assert_ok!(Tasks::submit_result(
					RawOrigin::Signed([0; 32].into()).into(),
					task_id,
					cycle,
					mock_result_ok(0, task_id, cycle)
				));
				cycle += 1;
				total_results += 1;
			}
		}
		assert_eq!(total_results, mock_task.cycle);
	});
}

#[test]
fn schedule_tasks_assigns_tasks_to_least_assigned_shard() {
	new_test_ext().execute_with(|| {
		// register shard before task assignment
		for i in 0..10 {
			Shards::create_shard(
				Network::Ethereum,
				[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
				1,
			);
			ShardState::<Test>::insert(i, ShardStatus::Online);
		}
		// shard online triggers task assignment
		for i in (0..10).rev() {
			Tasks::shard_online(i, Network::Ethereum);
			for _ in 0..i {
				assert_ok!(Tasks::create_task(
					RawOrigin::Signed([0; 32].into()).into(),
					mock_task(Network::Ethereum, 5)
				));
			}
		}
		for i in 0..10 {
			assert_eq!(Tasks::get_shard_tasks(i).len() as u64, i);
		}
	});
}

#[test]
fn submit_task_result_inserts_at_input_cycle() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 5)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		let task_result = mock_result_ok(0, 0, 0);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
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
	let shard_id = 0;
	let task_id = 0;
	let task_cycle = 0;
	let task_hash = "mock_hash";
	let a: AccountId = A.into();
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed(a.clone()).into(),
			mock_payable(Network::Ethereum)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		assert_eq!(
			<TaskPhaseState<Test>>::get(task_id),
			TaskPhase::Write(pubkey_from_bytes([0u8; 32]))
		);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::submit_hash(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			task_hash.into()
		));
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Read(Some(task_hash.into())));
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
	});
}

#[test]
fn resume_failed_task_after_shard_offline() {
	let mock_error = mock_error_result(0, 0, 0);
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		// fails 3 time to turn task status to failed
		for _ in 0..3 {
			assert_ok!(Tasks::submit_error(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_error.clone()
			));
		}
		assert_eq!(Tasks::task_shard(0), Some(0));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Failed { error: mock_error }));
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, Network::Ethereum);
		assert_eq!(Tasks::task_shard(0), None);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 0));
		assert_eq!(Tasks::task_shard(0), Some(0));
	});
}

#[test]
fn submit_signature_inserts_signature_into_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
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
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
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
fn register_gateway_fails_if_bootstrap_shard_is_offline() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),),
			Error::<Test>::BootstrapShardMustBeOnline
		);
	});
}

#[test]
fn register_gateway_emits_event() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		System::assert_last_event(
			Event::<Test>::GatewayRegistered(Network::Ethereum, [0u8; 20].to_vec()).into(),
		);
	});
}

#[test]
fn register_gateway_updates_shard_registered_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
	});
}

#[test]
fn register_gateway_updates_gateway_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_eq!(Gateway::<Test>::get(Network::Ethereum), Some([0u8; 20].to_vec()));
	});
}

#[test]
fn shard_online_starts_register_shard_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: None,
				network: Network::Ethereum,
				function: Function::RegisterShard { shard_id: 0 },
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
				shard_size: 3,
			}
		);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		// register gateway to register shard
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		//when a register shard task is complete the shard is marked as registered
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
	});
}

#[test]
fn register_gateway_completes_register_shard_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Completed));
		// insert shard public key to match mock result signature
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
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
fn shard_offline_starts_unregister_shard_task_and_unregisters_shard_immediately() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		// register gateway registers shard
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, Network::Ethereum);
		// shard not registered
		assert_eq!(ShardRegistered::<Test>::get(0), None);
		assert_eq!(
			Tasks::tasks(1).unwrap(),
			TaskDescriptor {
				owner: None,
				network: Network::Ethereum,
				function: Function::UnregisterShard { shard_id: 0 },
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
				shard_size: 3,
			}
		);
		assert_eq!(Tasks::task_state(1), Some(TaskStatus::Created));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		// complete task to unregister shard
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			1,
			0,
			mock_result_ok(0, 1, 0)
		));
		assert_eq!(Tasks::task_state(1), Some(TaskStatus::Completed));
	});
}

#[test]
fn shard_offline_stops_pending_register_shard_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Created));
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, Network::Ethereum);
		assert_eq!(Tasks::task_state(0), Some(TaskStatus::Stopped));
		// shard not registered
		assert_eq!(ShardRegistered::<Test>::get(0), None);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_noop!(
			Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),
			Error::<Test>::BootstrapShardMustBeOnline
		);
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, Network::Ethereum);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20].to_vec(),));
		// task to register 1st shard fails because it was stopped by `shard_offline`
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0; 32].into()).into(),
				0,
				0,
				mock_result_ok(1, 0, 0)
			),
			Error::<Test>::TaskStopped
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

#[test]
fn cannot_fund_task_beyond_caller_balance() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::create_task(
				RawOrigin::Signed([2; 32].into()).into(),
				mock_task(Network::Ethereum, 1)
			),
			sp_runtime::DispatchError::Token(sp_runtime::TokenError::FundsUnavailable,),
		);
	});
}

#[test]
fn task_may_not_be_funded_by_caller_without_balance() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::create_task(
				RawOrigin::Signed([2; 32].into()).into(),
				mock_task(Network::Ethereum, 1)
			),
			sp_runtime::DispatchError::Token(sp_runtime::TokenError::FundsUnavailable,),
		);
	});
}

#[test]
fn fund_task_fails_if_task_not_created() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::fund_task(RawOrigin::Signed([1; 32].into()).into(), 0, 100, 0),
			Error::<Test>::UnknownTask
		);
	});
}

#[test]
fn fund_task_correctly_funds_task() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		let mut task_balance = Tasks::task_balance(0);
		for i in 100..110 {
			assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, i, 0));
			task_balance = task_balance.saturating_add(i);
			assert_eq!(Tasks::task_balance(0), task_balance);
		}
	});
}

#[test]
fn fund_task_already_funded_emits_event() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		let mut task_balance = Tasks::task_balance(0);
		let balance_added = 100;
		assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, balance_added, 0));
		task_balance = task_balance.saturating_add(balance_added);
		System::assert_last_event(Event::<Test>::TaskFunded(0, task_balance).into());
	});
}

#[test]
fn fund_task_resumes_unfunded_stopped_task_iff_new_balance_above_min() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			TaskDescriptorParams {
				network: Network::Ethereum,
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				funds: 5,
				shard_size: 3,
			}
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, 9, 0));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Stopped));
		assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, 1, 0));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
	});
}

#[test]
fn fund_task_only_callable_by_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_noop!(
			Tasks::fund_task(RawOrigin::Signed([1; 32].into()).into(), 0, 100, 0),
			Error::<Test>::InvalidOwner
		);
		assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, 100, 0));
	});
}

#[test]
fn may_add_funds_for_task_not_funded_from_creation() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			TaskDescriptorParams {
				network: Network::Ethereum,
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				funds: 0,
				shard_size: 3,
			}
		));
		assert_eq!(Tasks::task_balance(0), 0);
		assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, 10, 0));
		assert_eq!(Tasks::task_balance(0), 10);
	});
}

#[test]
fn resume_read_task_fails_if_task_balance_below_min() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			TaskDescriptorParams {
				network: Network::Ethereum,
				cycle: 1,
				start: 0,
				period: 1,
				timegraph: None,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				funds: 0,
				shard_size: 3,
			}
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		TaskPhaseState::<Test>::insert(0, TaskPhase::Read(None));
		assert_noop!(
			Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 0),
			Error::<Test>::InvalidTaskState,
		);
		assert_ok!(Tasks::fund_task(RawOrigin::Signed([0; 32].into()).into(), 0, 10, 0));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
	});
}

#[test]
fn submit_hash_stops_unfunded_tasks() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			TaskDescriptorParams {
				network: Network::Ethereum,
				cycle: 1,
				start: 0,
				period: 0,
				timegraph: None,
				function: Function::EvmCall {
					address: Default::default(),
					input: Default::default(),
					amount: 0,
				},
				funds: 0,
				shard_size: 3,
			}
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, Network::Ethereum);
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		assert_ok!(Tasks::submit_hash(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			0,
			"mock_hash".into()
		));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Stopped));
	});
}

#[test]
fn set_read_task_reward_only_callable_by_root() {
	new_test_ext().execute_with(|| {
		assert_eq!(Tasks::network_read_reward(Network::Ethereum), 0);
		assert_ok!(Tasks::set_read_task_reward(RawOrigin::Root.into(), Network::Ethereum, 100,));
		assert_noop!(
			Tasks::set_read_task_reward(
				RawOrigin::Signed([0; 32].into()).into(),
				Network::Ethereum,
				100,
			),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_read_task_reward_updates_storage_and_emits_event() {
	new_test_ext().execute_with(|| {
		assert_eq!(Tasks::network_read_reward(Network::Ethereum), 0);
		assert_ok!(Tasks::set_read_task_reward(RawOrigin::Root.into(), Network::Ethereum, 100,));
		assert_eq!(Tasks::network_read_reward(Network::Ethereum), 100);
		System::assert_last_event(Event::<Test>::ReadTaskRewardSet(Network::Ethereum, 100).into());
	});
}

#[test]
fn stop_task_returns_task_balance_to_owner() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(Balances::free_balance(&[0; 32].into()), 9999999900);
		assert_eq!(Tasks::task_balance(0), 100);
		assert_ok!(Tasks::stop_task(RawOrigin::Root.into(), 0));
		assert_eq!(Balances::free_balance(&[0; 32].into()), 10000000000);
		assert_eq!(Tasks::task_balance(0), 0);
	});
}

#[test]
fn stop_task_enables_owner_to_drain_task_balance_at_will() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_eq!(Balances::free_balance(&[0; 32].into()), 9999999900);
		assert_eq!(Tasks::task_balance(0), 100);
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(Balances::free_balance(&[0; 32].into()), 10000000000);
		assert_eq!(Tasks::task_balance(0), 0);
		System::assert_last_event(Event::<Test>::TaskStopped(0).into());
	});
}

#[test]
fn resume_task_transfers_amount_to_task() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		assert_ok!(Tasks::stop_task(RawOrigin::Signed([0; 32].into()).into(), 0));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Stopped));
		assert_ok!(Tasks::resume_task(RawOrigin::Signed([0; 32].into()).into(), 0, 0, 10));
		assert_eq!(TaskState::<Test>::get(0), Some(TaskStatus::Created));
		System::assert_last_event(Event::<Test>::TaskResumed(0).into());
	});
}

#[test]
fn read_task_reward_goes_to_all_shard_members() {
	let shard_id = 0;
	let task_id = 0;
	let task_cycle = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_task(Network::Ethereum, 1)
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let mut balances = vec![];
		for member in [[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		let mut i = 0;
		for member in [[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec() {
			assert_eq!(
				Balances::free_balance(&member) - balances[i],
				<Test as crate::Config>::BaseReadReward::get()
			);
			i += 1;
		}
	});
}

#[test]
fn read_task_completion_clears_payout_storage() {
	let shard_id = 0;
	let task_id = 0;
	let mut task_cycle = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_task(Network::Ethereum, 2)
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(TotalPayout::<Test>::get(task_id), 0);
		assert!(SignerPayout::<Test>::iter_prefix(task_id).collect::<Vec<_>>().is_empty());
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		let expected_per_member_payout: u128 = <Test as crate::Config>::BaseReadReward::get();
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), expected_per_member_payout);
		assert_eq!(TotalPayout::<Test>::get(task_id), expected_per_member_payout.saturating_mul(3));
		assert!(SignerPayout::<Test>::iter_prefix(task_id).collect::<Vec<_>>().is_empty());
		task_cycle += 1;
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(TotalPayout::<Test>::get(task_id), 0);
		assert!(SignerPayout::<Test>::iter_prefix(task_id).collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn write_task_reward_goes_to_submitter() {
	let shard_id = 0;
	let task_id = 0;
	let task_cycle = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_payable(Network::Ethereum)
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let mut balances = vec![];
		for member in [[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		let mut i = 1;
		// unchanged balances for non-submitter shard members
		for member in [[1u8; 32].into(), [2u8; 32].into()].to_vec() {
			assert_eq!(Balances::free_balance(&member), balances[i]);
			i += 1;
		}
		// submitter shard member received BaseWriteReward for submitting the
		// result for a write task
		assert_eq!(
			Balances::free_balance(&[0u8; 32].into()) - balances[0],
			<Test as crate::Config>::BaseWriteReward::get()
		);
	});
}

#[test]
fn write_task_payout_clears_storage() {
	let shard_id = 0;
	let task_id = 0;
	let mut task_cycle = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			TaskDescriptorParams {
				network: Network::Ethereum,
				cycle: 2,
				start: 0,
				period: 0,
				timegraph: None,
				function: Function::EvmCall {
					address: Default::default(),
					input: Default::default(),
					amount: 0,
				},
				funds: 100,
				shard_size: 3,
			}
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let signer: AccountId = [0u8; 32].into();
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(TotalPayout::<Test>::get(task_id), 0);
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(
			TotalPayout::<Test>::get(task_id),
			<Test as crate::Config>::BaseWriteReward::get()
		);
		assert_eq!(
			SignerPayout::<Test>::get(task_id, &signer),
			<Test as crate::Config>::BaseWriteReward::get()
		);
		task_cycle += 1;
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(TotalPayout::<Test>::get(task_id), 0);
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
	});
}

#[test]
fn send_message_reward_goes_to_all_shard_members() {
	let shard_id = 0;
	let task_id = 0;
	let task_cycle = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 1)
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),);
		let mut balances = vec![];
		for member in [[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		let mut i = 1;
		let send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		for member in [[1u8; 32].into(), [2u8; 32].into()].to_vec() {
			assert_eq!(Balances::free_balance(&member) - balances[i], send_message_reward);
			i += 1;
		}
		let send_message_and_write_reward: u128 =
			send_message_reward.saturating_add(<Test as crate::Config>::BaseWriteReward::get());
		assert_eq!(
			Balances::free_balance(&[0u8; 32].into()) - balances[0],
			send_message_and_write_reward
		);
	});
}

#[test]
fn send_message_payout_clears_storage() {
	let shard_id = 0;
	let task_id = 0;
	let mut task_cycle = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			Network::Ethereum,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_sign_task(Network::Ethereum, 2)
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20].to_vec(),),);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, [0u8; 64]),);
		let signer: AccountId = [0u8; 32].into();
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(TotalPayout::<Test>::get(task_id), 0);
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		let write_reward: u128 = <Test as crate::Config>::BaseWriteReward::get();
		let send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), send_message_reward);
		assert_eq!(
			TotalPayout::<Test>::get(task_id),
			send_message_reward.saturating_mul(3).saturating_add(write_reward)
		);
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), write_reward);
		task_cycle += 1;
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		assert_eq!(MemberPayout::<Test>::get(task_id, shard_id), 0);
		assert_eq!(TotalPayout::<Test>::get(task_id), 0);
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
	});
}
