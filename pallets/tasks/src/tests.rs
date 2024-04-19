use crate::mock::*;
use crate::{
	Error, Event, Gateway, GatewayReset, NetworkReadReward, NetworkSendMessageReward,
	NetworkShards, NetworkWriteReward, ShardRegistered, ShardTasks, SignerPayout, TaskHash,
	TaskIdCounter, TaskOutput, TaskPhaseState, TaskRewardConfig, TaskShard, TaskSignature,
	TaskSigner, UnassignedTasks,
};
use frame_support::traits::Get;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use sp_runtime::Saturating;
use time_primitives::{
	AccountId, Function, GmpParams, Message, Msg, NetworkId, Payload, PublicKey, RewardConfig,
	ShardId, ShardStatus, ShardsInterface, TaskDescriptor, TaskDescriptorParams, TaskExecution,
	TaskId, TaskPhase, TaskResult, TasksInterface,
};

fn shard() -> [AccountId; 3] {
	[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()]
}

fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

const ETHEREUM: NetworkId = 0;
const A: [u8; 32] = [1u8; 32];

fn mock_task(network: NetworkId) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		start: 0,
		function: Function::EvmViewCall {
			address: Default::default(),
			input: Default::default(),
		},
		funds: 100,
		shard_size: 3,
	}
}

fn mock_sign_task(network: NetworkId) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		start: 0,
		function: Function::SendMessage { msg: Msg::default() },
		funds: 100,
		shard_size: 3,
	}
}

fn mock_payable(network: NetworkId) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		start: 0,
		function: Function::EvmCall {
			address: Default::default(),
			input: Default::default(),
			amount: 0,
			gas_limit: None,
		},
		funds: 100,
		shard_size: 3,
	}
}

fn mock_result_ok(shard_id: ShardId, task_id: TaskId) -> TaskResult {
	// these values are taken after running a valid instance of submitting result
	let hash = [
		11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91, 0,
		219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
	];
	let payload = Payload::Hashed(hash);
	let signature = MockTssSigner::new().sign(&payload.bytes(task_id)).to_bytes();
	TaskResult { shard_id, payload, signature }
}

fn mock_submit_sig(function: Function) -> [u8; 64] {
	let tss_public_key = MockTssSigner::new().public_key();
	let gmp_params = GmpParams {
		network_id: ETHEREUM,
		tss_public_key,
		gateway_contract: [0u8; 20].into(),
	};
	let payload: Vec<u8> = match function {
		Function::RegisterShard { .. } => {
			let tss_pubkey = MockTssSigner::new().public_key();
			Message::update_keys([], [tss_pubkey]).to_eip712_bytes(&gmp_params).into()
		},
		Function::UnregisterShard { .. } => {
			let tss_pubkey = MockTssSigner::new().public_key();
			Message::update_keys([tss_pubkey], []).to_eip712_bytes(&gmp_params).into()
		},
		Function::SendMessage { msg } => Message::gmp(msg).to_eip712_bytes(&gmp_params).into(),
		_ => Default::default(),
	};
	MockTssSigner::new().sign(&payload).to_bytes()
}

fn mock_error_result(shard_id: ShardId, task_id: TaskId) -> TaskResult {
	// these values are taken after running a valid instance of submitting error
	let payload = Payload::Error("Invalid input length".into());
	let signature = MockTssSigner::new().sign(&payload.bytes(task_id)).to_bytes();
	TaskResult { shard_id, payload, signature }
}

#[test]
fn test_create_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		System::assert_last_event(Event::<Test>::TaskAssigned(0, 0).into());
		assert!(System::events().iter().any(|e| e.event == Event::<Test>::TaskCreated(0).into()));
		assert_eq!(Tasks::get_shard_tasks(0), vec![TaskExecution::new(0, TaskPhase::Read)]);
		let mut read_task_reward: u128 = <Test as crate::Config>::BaseReadReward::get();
		read_task_reward =
			read_task_reward.saturating_add(NetworkReadReward::<Test>::get(ETHEREUM));
		let mut write_task_reward: u128 = <Test as crate::Config>::BaseWriteReward::get();
		write_task_reward =
			write_task_reward.saturating_add(NetworkWriteReward::<Test>::get(ETHEREUM));
		let mut send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		send_message_reward =
			send_message_reward.saturating_add(NetworkSendMessageReward::<Test>::get(ETHEREUM));
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
		let task_result = mock_result_ok(0, 0);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			task_result.clone()
		));
		System::assert_last_event(Event::<Test>::TaskResult(0, task_result).into());
	});
}

#[test]
fn create_task_increments_task_id_counter() {
	new_test_ext().execute_with(|| {
		for i in 0..11 {
			Shards::create_shard(
				ETHEREUM,
				[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
				1,
			);
			ShardState::<Test>::insert(0, ShardStatus::Online);
			Tasks::shard_online(0, ETHEREUM);
			assert_ok!(Tasks::create_task(
				RawOrigin::Signed([0; 32].into()).into(),
				mock_task(ETHEREUM)
			));
			assert_eq!(TaskIdCounter::<Test>::get(), i.saturating_plus_one());
		}
	});
}

#[test]
fn create_task_fails_sans_shards() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), mock_task(ETHEREUM)),
			Error::<Test>::MatchingShardNotOnline
		);
	});
}

#[test]
fn task_unassigned_if_all_shards_offline() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: Some([0; 32].into()),
				network: ETHEREUM,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				start: 0,
				shard_size: 3,
			}
		);
		Tasks::shard_offline(0, ETHEREUM);
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>(), vec![(ETHEREUM, 0, ())]);
	});
}

#[test]
fn task_auto_assigned_if_shard_online() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: Some([0; 32].into()),
				network: ETHEREUM,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				start: 0,
				shard_size: 3,
			}
		);
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 0);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
	});
}

#[test]
fn task_auto_assigned_if_shard_joins_after() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: Some([0; 32].into()),
				network: ETHEREUM,
				function: Function::EvmViewCall {
					address: Default::default(),
					input: Default::default(),
				},
				start: 0,
				shard_size: 3,
			}
		);
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 0);
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
	});
}

#[test]
fn shard_online_inserts_network_shards() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		assert!(NetworkShards::<Test>::get(ETHEREUM, 0).is_none());
		Tasks::shard_online(0, ETHEREUM);
		assert!(NetworkShards::<Test>::get(ETHEREUM, 0).is_some());
	});
}

#[test]
fn shard_offline_removes_network_shards() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Tasks::shard_online(0, ETHEREUM);
		assert!(NetworkShards::<Test>::get(ETHEREUM, 0).is_some());
		Tasks::shard_offline(0, ETHEREUM);
		assert!(NetworkShards::<Test>::get(ETHEREUM, 0).is_none());
	});
}

#[test]
fn shard_offline_removes_tasks() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![0]);
		assert!(UnassignedTasks::<Test>::iter()
			.map(|(_, t, _)| t)
			.collect::<Vec<_>>()
			.is_empty());
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0));
		assert_eq!(ShardTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(), vec![1, 0]);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		// put shard 2 online to be assigned UnregisterShard task for new offline shard
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, ETHEREUM);
		Tasks::shard_offline(0, ETHEREUM);
		assert_eq!(
			UnassignedTasks::<Test>::iter().map(|(_, t, _)| t).collect::<Vec<_>>(),
			vec![3, 2]
		);
	});
}

#[test]
fn shard_offline_assigns_tasks_if_other_shard_online() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(0, 0)]
		);
		assert!(UnassignedTasks::<Test>::iter()
			.map(|(_, t, _)| t)
			.collect::<Vec<_>>()
			.is_empty());
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, ETHEREUM);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, ETHEREUM);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty(),);
		assert_eq!(
			ShardTasks::<Test>::iter().map(|(s, t, _)| (s, t)).collect::<Vec<_>>(),
			vec![(1, 0)]
		);
	});
}

#[test]
fn submit_completed_result_purges_task_from_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			mock_result_ok(0, 0)
		));
		assert_eq!(ShardTasks::<Test>::iter().collect::<Vec<_>>().len(), 1);
		assert!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

// TODO: is this change in behavior intended? intended to not drop failed tasks but test was incorrect before
#[test]
fn shard_offline_drops_failed_tasks() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		let sign_task = mock_sign_task(ETHEREUM);
		assert_ok!(
			Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone(),)
		);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let pub_key = MockTssSigner::new().public_key();
		ShardCommitment::<Test>::insert(0, vec![pub_key]);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0u8; 32].into()).into(), 0, [0; 32],));
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			mock_error_result(0, 0)
		));
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_offline(0, ETHEREUM);
		assert!(ShardTasks::<Test>::iter().collect::<Vec<_>>().is_empty());
		assert_eq!(UnassignedTasks::<Test>::iter().collect::<Vec<_>>().len(), 1);
	});
}

#[test]
fn submit_task_error_is_task_failure() {
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone()));
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0u8; 32].into()).into(), 0, [0; 32],));
		let error = mock_error_result(0, 0);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			error.clone()
		));
	});
}

#[test]
fn task_moved_on_shard_offline() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Shards::create_shard(
			ETHEREUM,
			[[1u8; 32].into(), [2u8; 32].into(), [3u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_eq!(Tasks::get_shard_tasks(0), vec![TaskExecution::new(0, TaskPhase::default()),]);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, ETHEREUM);
		Tasks::shard_offline(0, ETHEREUM);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		assert_eq!(Tasks::get_shard_tasks(0), vec![]);
		assert_eq!(Tasks::get_shard_tasks(1), vec![TaskExecution::new(0, TaskPhase::default()),]);
	});
}

#[test]
fn schedule_tasks_assigns_tasks_to_least_assigned_shard() {
	new_test_ext().execute_with(|| {
		// register shard before task assignment
		for i in 0..10 {
			Shards::create_shard(
				ETHEREUM,
				[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
				1,
			);
			ShardState::<Test>::insert(i, ShardStatus::Online);
		}
		// shard online triggers task assignment
		for i in (0..10).rev() {
			Tasks::shard_online(i, ETHEREUM);
			for _ in 0..i {
				assert_ok!(Tasks::create_task(
					RawOrigin::Signed([0; 32].into()).into(),
					mock_task(ETHEREUM)
				));
			}
		}
		for i in 0..10 {
			assert_eq!(Tasks::get_shard_tasks(i).len() as u64, i);
		}
	});
}

#[test]
fn submit_task_result_inserts_task_output() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		let task_result = mock_result_ok(0, 0);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			0,
			task_result.clone()
		));
		assert!(TaskOutput::<Test>::get(0).is_some());
		assert!(TaskOutput::<Test>::get(1).is_none());
		System::assert_last_event(Event::<Test>::TaskResult(0, task_result).into());
	});
}

#[test]
fn payable_task_smoke() {
	let shard_id = 0;
	let task_id = 0;
	let task_hash = [0; 32];
	let a: AccountId = A.into();
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(RawOrigin::Signed(a.clone()).into(), mock_payable(ETHEREUM)));
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Write,);
		assert_eq!(<TaskSigner<Test>>::get(task_id), Some(pubkey_from_bytes([0u8; 32])));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::submit_hash(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_hash
		));
		assert_eq!(<TaskPhaseState<Test>>::get(task_id), TaskPhase::Read);
		assert_eq!(<TaskHash<Test>>::get(task_id), Some(task_hash));
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
	});
}

#[test]
fn submit_signature_inserts_signature_into_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		let sign_task = mock_sign_task(ETHEREUM);
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone()));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_eq!(TaskSignature::<Test>::get(0), Some(sig));
	});
}

#[test]
fn submit_signature_fails_when_task_dne() {
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		let sig = mock_submit_sig(sign_task.function);
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig),
			Error::<Test>::UnknownTask
		);
	});
}

#[test]
fn submit_signature_fails_if_not_sign_phase() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		let sign_task = mock_task(ETHEREUM);
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone()));
		let sig = mock_submit_sig(sign_task.function);
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig),
			Error::<Test>::NotSignPhase
		);
	});
}

#[test]
fn submit_signature_fails_if_unassigned() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		let sign_task = mock_sign_task(ETHEREUM);
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone()));
		let sig = mock_submit_sig(sign_task.function);
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig),
			Error::<Test>::UnassignedTask
		);
	});
}

#[test]
fn submit_signature_fails_after_called_once() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		let sign_task = mock_sign_task(ETHEREUM);
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone()));
		assert_ok!(Tasks::create_task(RawOrigin::Signed([0; 32].into()).into(), sign_task.clone()));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_noop!(
			Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig),
			Error::<Test>::TaskSigned
		);
	});
}

#[test]
fn register_gateway_fails_if_not_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::register_gateway(RawOrigin::Signed([0; 32].into()).into(), 1, [0u8; 20], 0),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn register_gateway_fails_if_bootstrap_shard_is_offline() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20], 0),
			Error::<Test>::BootstrapShardMustBeOnline
		);
	});
}

#[test]
fn register_gateway_emits_event() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert!(System::events()
			.iter()
			.any(|e| e.event == Event::<Test>::GatewayRegistered(ETHEREUM, [0u8; 20], 0).into()));
	});
}

#[test]
fn register_gateway_resets_timeout_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_eq!(GatewayReset::<Test>::get(0), None);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(GatewayReset::<Test>::get(0), Some(1));
	});
}

#[test]
fn register_gateway_updates_shard_registered_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
	});
}

#[test]
fn register_gateway_updates_gateway_storage() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(Gateway::<Test>::get(ETHEREUM), Some([0u8; 20]));
	});
}

#[test]
fn register_gateway_sets_non_read_gmp_tasks_to_sign_phase() {
	const NUM_TASKS: u64 = 5;
	const SHARD_ID: u64 = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(SHARD_ID, ShardStatus::Online);
		Tasks::shard_online(SHARD_ID, ETHEREUM);
		for i in 0..NUM_TASKS {
			assert_ok!(Tasks::create_task(
				RawOrigin::Signed([0; 32].into()).into(),
				mock_sign_task(ETHEREUM)
			));
			TaskShard::<Test>::insert(i, SHARD_ID);
			TaskPhaseState::<Test>::insert(i, TaskPhase::Write);
		}
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), SHARD_ID, [0u8; 20], 0),);
		for i in 0..NUM_TASKS {
			assert_eq!(TaskPhaseState::<Test>::get(i), TaskPhase::Sign);
		}
	});
}

#[test]
fn shard_online_starts_register_shard_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		// register gateway to register shard
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		//when a register shard task is complete the shard is marked as registered
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
	});
}

#[test]
fn register_gateway_starts_register_shard_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		Tasks::shard_online(1, ETHEREUM);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
	});
}

#[test]
#[ignore]
fn shard_offline_starts_unregister_shard_task_and_unregisters_shard_immediately() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		// register gateway registers shard
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, ETHEREUM);
		// shard not registered
		assert_eq!(ShardRegistered::<Test>::get(0), None);
		assert_eq!(
			Tasks::tasks(0).unwrap(),
			TaskDescriptor {
				owner: None,
				network: ETHEREUM,
				function: Function::UnregisterShard { shard_id: 0 },
				start: 0,
				shard_size: 3,
			}
		);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, ETHEREUM);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20], 0),);
		ShardCommitment::<Test>::insert(1, vec![MockTssSigner::new().public_key()]);
		let sig = mock_submit_sig(Function::UnregisterShard { shard_id: 0 });
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 1, sig,),);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), 1, [0; 32],),);
		// complete task to unregister shard
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0; 32].into()).into(),
			1,
			mock_result_ok(1, 1)
		));
	});
}

#[test]
fn shard_offline_stops_pending_register_shard_task() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		ShardState::<Test>::insert(0, ShardStatus::Offline);
		Tasks::shard_offline(0, ETHEREUM);
		// shard not registered
		assert_eq!(ShardRegistered::<Test>::get(0), None);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_noop!(
			Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),
			Error::<Test>::BootstrapShardMustBeOnline
		);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(1, ShardStatus::Online);
		Tasks::shard_online(1, ETHEREUM);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 1, [0u8; 20], 0));
	});
}

#[test]
fn shard_offline_does_not_schedule_unregister_if_shard_not_registered() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		Tasks::shard_online(0, ETHEREUM);
		Tasks::shard_offline(0, ETHEREUM);
		assert!(Tasks::tasks(1).is_none());
		// task to unregister shard does not exist
		assert!(Tasks::tasks(1).is_none());
		assert_noop!(
			Tasks::submit_result(RawOrigin::Signed([0; 32].into()).into(), 1, mock_result_ok(0, 1)),
			Error::<Test>::UnknownTask
		);
	});
}

#[test]
fn cannot_fund_task_beyond_caller_balance() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_noop!(
			Tasks::create_task(RawOrigin::Signed([2; 32].into()).into(), mock_task(ETHEREUM)),
			sp_runtime::DispatchError::Token(sp_runtime::TokenError::FundsUnavailable,),
		);
	});
}

#[test]
fn task_may_not_be_funded_by_caller_without_balance() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		assert_noop!(
			Tasks::create_task(RawOrigin::Signed([2; 32].into()).into(), mock_task(ETHEREUM)),
			sp_runtime::DispatchError::Token(sp_runtime::TokenError::FundsUnavailable,),
		);
	});
}

#[test]
fn set_read_task_reward_only_callable_by_root() {
	new_test_ext().execute_with(|| {
		assert_eq!(Tasks::network_read_reward(ETHEREUM), 0);
		assert_ok!(Tasks::set_read_task_reward(RawOrigin::Root.into(), ETHEREUM, 100,));
		assert_noop!(
			Tasks::set_read_task_reward(RawOrigin::Signed([0; 32].into()).into(), ETHEREUM, 100,),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_read_task_reward_updates_storage_and_emits_event() {
	new_test_ext().execute_with(|| {
		assert_eq!(Tasks::network_read_reward(ETHEREUM), 0);
		assert_ok!(Tasks::set_read_task_reward(RawOrigin::Root.into(), ETHEREUM, 100,));
		assert_eq!(Tasks::network_read_reward(ETHEREUM), 100);
		System::assert_last_event(Event::<Test>::ReadTaskRewardSet(ETHEREUM, 100).into());
	});
}

#[test]
fn read_task_reward_goes_to_all_shard_members() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let mut balances = vec![];
		for member in shard() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		for (i, member) in shard().into_iter().enumerate() {
			assert_eq!(
				Balances::free_balance(&member) - balances[i],
				<Test as crate::Config>::BaseReadReward::get()
			);
		}
	});
}

#[test]
fn read_task_completion_clears_payout_storage() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert!(SignerPayout::<Test>::iter_prefix(task_id).collect::<Vec<_>>().is_empty());
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		assert!(SignerPayout::<Test>::iter_prefix(task_id).collect::<Vec<_>>().is_empty());
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		assert!(SignerPayout::<Test>::iter_prefix(task_id).collect::<Vec<_>>().is_empty());
	});
}

#[test]
/// Integration test for reward payout of send message + read reward for all
/// and an additional write reward for the signer.
/// Also checks that SignerPayout storage is cleared upon payout.
fn send_message_for_all_plus_write_reward_for_signer() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		let mut balances = vec![];
		for member in shard() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), 0, [0; 32]),);
		let signer: AccountId = [0; 32].into();
		assert_eq!(
			SignerPayout::<Test>::get(task_id, &signer),
			<Test as crate::Config>::BaseWriteReward::get()
		);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		// payout storage cleared
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		let read_message_reward: u128 = <Test as crate::Config>::BaseReadReward::get();
		let send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		for (i, member) in shard().into_iter().enumerate() {
			let every_member_reward = read_message_reward.saturating_add(send_message_reward);
			if i == 0 {
				let send_message_and_write_reward: u128 = every_member_reward
					.saturating_add(<Test as crate::Config>::BaseWriteReward::get());
				assert_eq!(
					Balances::free_balance(&member) - balances[0],
					send_message_and_write_reward
				);
			} else {
				assert_eq!(Balances::free_balance(&member) - balances[i], every_member_reward);
			}
		}
	});
}

#[test]
fn send_message_payout_clears_storage() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		let sign_task = mock_sign_task(ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), 0, [0; 32]),);
		let signer: AccountId = [0u8; 32].into();
		let write_reward: u128 = <Test as crate::Config>::BaseWriteReward::get();
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), write_reward);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
	});
}

#[test]
/// Test read phase timeout to assign to new shard
/// NOTE write phase timeout test in runtime integration tests
fn read_phase_times_out_and_reassigns_for_read_only_task() {
	let shards_count = 2;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		for i in 1..shards_count {
			Shards::create_shard(
				ETHEREUM,
				[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
				1,
			);
			ShardState::<Test>::insert(i, ShardStatus::Online);
			Tasks::shard_online(i, ETHEREUM);
			ShardCommitment::<Test>::insert(i, vec![MockTssSigner::new().public_key()]);
		}
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(TaskShard::<Test>::get(0), Some(0));
		assert_eq!(ShardTasks::<Test>::get(0, 0), Some(()));
		assert_eq!(ShardTasks::<Test>::get(1, 0), None);
		let mut next =
			<<Test as crate::Config>::ReadPhaseTimeout as Get<u64>>::get().saturating_plus_one();
		roll_to(next);
		assert_eq!(TaskShard::<Test>::get(0), Some(1));
		assert_eq!(ShardTasks::<Test>::get(0, 0), None);
		assert_eq!(ShardTasks::<Test>::get(1, 0), Some(()));
		next = next.saturating_add(<<Test as crate::Config>::ReadPhaseTimeout as Get<u64>>::get());
		roll_to(next);
		assert_eq!(TaskShard::<Test>::get(0), Some(0));
		assert_eq!(ShardTasks::<Test>::get(0, 0), Some(()));
		assert_eq!(ShardTasks::<Test>::get(1, 0), None);
	});
}

#[test]
fn read_phase_times_out_for_sign_task_in_read_phase() {
	let shards_count = 2;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(0, ShardStatus::Online);
		Tasks::shard_online(0, ETHEREUM);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let sign_task = mock_sign_task(ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		for i in 1..shards_count {
			Shards::create_shard(
				ETHEREUM,
				[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
				1,
			);
			ShardState::<Test>::insert(i, ShardStatus::Online);
			Tasks::shard_online(i, ETHEREUM);
			ShardCommitment::<Test>::insert(i, vec![MockTssSigner::new().public_key()]);
		}
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), 0, [0; 32]),);
		assert_eq!(TaskShard::<Test>::get(0), Some(0));
		assert_eq!(ShardTasks::<Test>::get(0, 0), Some(()));
		assert_eq!(ShardTasks::<Test>::get(1, 0), None);
		let mut next =
			<<Test as crate::Config>::ReadPhaseTimeout as Get<u64>>::get().saturating_plus_one();
		roll_to(next);
		assert_eq!(TaskShard::<Test>::get(0), Some(1));
		assert_eq!(ShardTasks::<Test>::get(0, 0), None);
		assert_eq!(ShardTasks::<Test>::get(1, 0), Some(()));
		next = next.saturating_add(<<Test as crate::Config>::ReadPhaseTimeout as Get<u64>>::get());
		roll_to(next);
		assert_eq!(TaskShard::<Test>::get(0), Some(0));
		assert_eq!(ShardTasks::<Test>::get(0, 0), Some(()));
		assert_eq!(ShardTasks::<Test>::get(1, 0), None);
	});
}

#[test]
fn submit_result_fails_if_not_read_phase() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0u8; 32].into()).into(),
				task_id,
				mock_result_ok(shard_id, task_id)
			),
			Error::<Test>::NotReadPhase
		);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0u8; 32].into()).into(),
				task_id,
				mock_result_ok(shard_id, task_id)
			),
			Error::<Test>::NotReadPhase
		);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), 0, [0; 32]),);
	});
}

#[test]
fn write_reward_depreciates_correctly() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		let mut balances = vec![];
		for member in shard() {
			balances.push(Balances::free_balance(&member));
		}
		// get RewardConfig
		let reward_config = TaskRewardConfig::<Test>::get(task_id).unwrap();
		assert_eq!(System::block_number(), 1);
		roll_to(reward_config.depreciation_rate.blocks * 2);
		assert_eq!(System::block_number(), reward_config.depreciation_rate.blocks * 2);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), task_id, [0; 32]),);
		let signer: AccountId = [0; 32].into();
		let reward_sans_depreciation: u128 = <Test as crate::Config>::BaseWriteReward::get();
		let expected_write_reward = reward_sans_depreciation
			.saturating_sub(reward_config.depreciation_rate.percent * reward_sans_depreciation);
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), expected_write_reward);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		// payout storage cleared
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		let read_message_reward: u128 = <Test as crate::Config>::BaseReadReward::get();
		let send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		for (i, member) in shard().into_iter().enumerate() {
			let every_member_reward = read_message_reward.saturating_add(send_message_reward);
			if i == 0 {
				let send_message_and_write_reward: u128 =
					every_member_reward.saturating_add(expected_write_reward);
				assert_eq!(
					Balances::free_balance(&member) - balances[0],
					send_message_and_write_reward
				);
			} else {
				assert_eq!(Balances::free_balance(&member) - balances[i], every_member_reward);
			}
		}
	});
}

#[test]
fn submit_err_fails_if_not_read_phase() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0u8; 32].into()).into(),
				task_id,
				mock_error_result(shard_id, task_id)
			),
			Error::<Test>::NotReadPhase
		);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		assert_noop!(
			Tasks::submit_result(
				RawOrigin::Signed([0u8; 32].into()).into(),
				task_id,
				mock_error_result(shard_id, task_id)
			),
			Error::<Test>::NotReadPhase
		);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), 0, [0; 32]),);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_error_result(shard_id, task_id)
		));
	});
}

#[test]
fn write_reward_eventually_depreciates_to_lower_bound_1() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		let mut balances = vec![];
		for member in shard() {
			balances.push(Balances::free_balance(&member));
		}
		// get RewardConfig
		let reward_config = TaskRewardConfig::<Test>::get(task_id).unwrap();
		assert_eq!(System::block_number(), 1);
		roll_to(reward_config.depreciation_rate.blocks * 200);
		assert_eq!(System::block_number(), reward_config.depreciation_rate.blocks * 200);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), task_id, [0; 32]),);
		let signer: AccountId = [0; 32].into();
		// asymptotic lower bound for all depreciation is the lowest unit
		let expected_write_reward = 1u128;
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), expected_write_reward);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		// payout storage cleared
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		let read_message_reward: u128 = <Test as crate::Config>::BaseReadReward::get();
		let send_message_reward: u128 = <Test as crate::Config>::BaseSendMessageReward::get();
		for (i, member) in shard().into_iter().enumerate() {
			let every_member_reward = read_message_reward.saturating_add(send_message_reward);
			if i == 0 {
				let send_message_and_write_reward: u128 =
					every_member_reward.saturating_add(expected_write_reward);
				assert_eq!(
					Balances::free_balance(&member) - balances[0],
					send_message_and_write_reward
				);
			} else {
				assert_eq!(Balances::free_balance(&member) - balances[i], every_member_reward);
			}
		}
	});
}

#[test]
fn read_send_message_rewards_depreciate_correctly() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		let mut balances = vec![];
		for member in shard() {
			balances.push(Balances::free_balance(&member));
		}
		// get RewardConfig
		let reward_config = TaskRewardConfig::<Test>::get(task_id).unwrap();
		assert_eq!(System::block_number(), 1);
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), task_id, [0; 32]),);
		let signer: AccountId = [0; 32].into();
		let expected_write_reward: u128 = <Test as crate::Config>::BaseWriteReward::get();
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), expected_write_reward);
		assert_eq!(System::block_number(), 1);
		roll_to(reward_config.depreciation_rate.blocks * 2);
		assert_eq!(System::block_number(), reward_config.depreciation_rate.blocks * 2);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		// payout storage cleared
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		let read_reward_sans_depreciation: u128 = <Test as crate::Config>::BaseReadReward::get();
		let expected_read_reward: u128 = read_reward_sans_depreciation.saturating_sub(
			reward_config.depreciation_rate.percent * read_reward_sans_depreciation,
		);
		let send_msg_reward_sans_depreciation: u128 =
			<Test as crate::Config>::BaseSendMessageReward::get();
		let send_message_reward: u128 = send_msg_reward_sans_depreciation.saturating_sub(
			reward_config.depreciation_rate.percent * send_msg_reward_sans_depreciation,
		);
		for (i, member) in shard().into_iter().enumerate() {
			let every_member_reward = expected_read_reward.saturating_add(send_message_reward);
			if i == 0 {
				// signer
				assert_eq!(
					Balances::free_balance(&member) - balances[0],
					every_member_reward.saturating_add(expected_write_reward)
				);
			} else {
				assert_eq!(Balances::free_balance(&member) - balances[i], every_member_reward);
			}
		}
	});
}

#[test]
fn read_send_message_rewards_eventually_depreciate_to_lower_bound_1() {
	let shard_id = 0;
	let task_id = 0;
	new_test_ext().execute_with(|| {
		let sign_task = mock_sign_task(ETHEREUM);
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			sign_task.clone()
		));
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		let sig = mock_submit_sig(sign_task.function);
		assert_ok!(Tasks::submit_signature(RawOrigin::Signed([0; 32].into()).into(), 0, sig,),);
		let mut balances = vec![];
		for member in shard() {
			balances.push(Balances::free_balance(&member));
		}
		// get RewardConfig
		let reward_config = TaskRewardConfig::<Test>::get(task_id).unwrap();
		assert_ok!(Tasks::submit_hash(RawOrigin::Signed([0; 32].into()).into(), task_id, [0; 32],),);
		let signer: AccountId = [0; 32].into();
		// asymptotic lower bound for all depreciation is the lowest unit
		let expected_write_reward = <Test as crate::Config>::BaseWriteReward::get();
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), expected_write_reward);
		assert_eq!(System::block_number(), 1);
		roll_to(reward_config.depreciation_rate.blocks * 200);
		assert_eq!(System::block_number(), reward_config.depreciation_rate.blocks * 200);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			mock_result_ok(shard_id, task_id)
		));
		// payout storage cleared
		assert_eq!(SignerPayout::<Test>::get(task_id, &signer), 0);
		let read_message_reward: u128 = 1;
		let send_message_reward: u128 = 1;
		for (i, member) in shard().into_iter().enumerate() {
			let every_member_reward = read_message_reward.saturating_add(send_message_reward);
			if i == 0 {
				let send_message_and_write_reward: u128 =
					every_member_reward.saturating_add(expected_write_reward);
				assert_eq!(
					Balances::free_balance(&member) - balances[0],
					send_message_and_write_reward
				);
			} else {
				assert_eq!(Balances::free_balance(&member) - balances[i], every_member_reward);
			}
		}
	});
}

#[test]
#[ignore]
fn bench_result_helper() {
	fn bench_result(shard_id: ShardId, task_id: TaskId) -> ([u8; 33], TaskResult) {
		// these values are taken after running a valid instance of submitting result
		let hash = [
			11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91,
			0, 219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
		];
		let payload = Payload::Hashed(hash);
		let signer = MockTssSigner::new();
		let signature = signer.sign(&payload.bytes(task_id)).to_bytes();
		(signer.public_key(), TaskResult { shard_id, payload, signature })
	}
	println!("{:?}", bench_result(0, 0));
	//assert!(false);
}

#[test]
#[ignore]
fn bench_sig_helper() {
	fn bench_sig() -> ([u8; 33], [u8; 64]) {
		//let function = Function::SendMessage { msg: Msg::default() };
		let signer = MockTssSigner::new();
		let tss_public_key = signer.public_key();
		let gmp_params = GmpParams {
			network_id: ETHEREUM,
			tss_public_key,
			gateway_contract: [0u8; 20].into(),
		};
		let payload: Vec<u8> = Message::gmp(Msg::default()).to_eip712_bytes(&gmp_params).into();
		(tss_public_key, signer.sign(&payload).to_bytes())
	}
	println!("{:?}", bench_sig());
	//assert!(false);
}

#[test]
fn lock_gateway_if_less_than_one_shard_online() {
	let shard_id = 0;
	new_test_ext().execute_with(|| {
		Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, ETHEREUM);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_task(ETHEREUM)
		));
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0));
		Tasks::shard_offline(shard_id, ETHEREUM);
		// Remove gateway address
		assert!(Gateway::<Test>::get(ETHEREUM).is_none());
		// Emit `Event::GatewayLocked(Network)`
		System::assert_last_event(Event::<Test>::GatewayLocked(ETHEREUM).into());
		// Set all task output to failed
		assert_eq!(
			TaskOutput::<Test>::get(0).unwrap(),
			TaskResult {
				shard_id: 0,
				payload: Payload::Error("Gateway locked".into()),
				signature: [0u8; 64],
			}
		);
	});
}

#[test]
fn register_gateway_fails_previous_shard_registration_tasks() {
	new_test_ext().execute_with(|| {
		const NUM_SHARDS: u64 = 5;
		for i in 0..NUM_SHARDS {
			Shards::create_shard(
				ETHEREUM,
				[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
				1,
			);
			ShardState::<Test>::insert(i, ShardStatus::Online);
			Tasks::shard_online(i, ETHEREUM);
		}
		let mut expected_failed_tasks = Vec::new();
		for (task_id, task) in crate::Tasks::<Test>::iter() {
			if let Function::RegisterShard { shard_id } = task.function {
				expected_failed_tasks.push((shard_id, task_id));
			}
		}
		assert_ok!(Tasks::register_gateway(RawOrigin::Root.into(), 0, [0u8; 20], 0),);
		assert_eq!(ShardRegistered::<Test>::get(0), Some(()));
		for (shard_id, task_id) in expected_failed_tasks.iter() {
			assert_eq!(
				TaskOutput::<Test>::get(task_id),
				Some(TaskResult {
					shard_id: *shard_id,
					payload: Payload::Error("new gateway registered".into()),
					signature: [0u8; 64],
				})
			);
		}
	});
}
