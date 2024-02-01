//! Tests for reward computation and payout
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
		for member in shard_size_3() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		for (i, member) in shard_size_3().into_iter().enumerate() {
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
	let a: AccountId = [0u8; 32].into();
	new_test_ext().execute_with(|| {
		Shards::create_shard(Network::Ethereum, shard_size_3().to_vec(), 1);
		assert_ok!(Tasks::create_task(
			RawOrigin::Signed([0u8; 32].into()).into(),
			mock_payable(Network::Ethereum)
		));
		ShardState::<Test>::insert(shard_id, ShardStatus::Online);
		Tasks::shard_online(shard_id, Network::Ethereum);
		ShardCommitment::<Test>::insert(0, vec![MockTssSigner::new().public_key()]);
		let mut balances = vec![];
		for member in shard_size_3() {
			balances.push(Balances::free_balance(&member));
		}
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed(a.clone()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		let mut i = 1;
		// unchanged balances for non-submitter shard members
		for member in shard_size_2() {
			assert_eq!(Balances::free_balance(&member), balances[i]);
			i += 1;
		}
		// submitter shard member received BaseWriteReward for submitting the
		// result for a write task
		assert_eq!(
			Balances::free_balance(a) - balances[0],
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
	let a: AccountId = [0u8; 32].into();
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
		for member in shard_size_3() {
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
		for member in shard_size_2() {
			assert_eq!(Balances::free_balance(&member) - balances[i], send_message_reward);
			i += 1;
		}
		let send_message_and_write_reward: u128 =
			send_message_reward.saturating_add(<Test as crate::Config>::BaseWriteReward::get());
		assert_eq!(Balances::free_balance(a) - balances[0], send_message_and_write_reward);
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

#[test]
fn read_task_reward_depreciates_after_first_n_blocks() {
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
		for member in shard_size_3() {
			balances.push(Balances::free_balance(&member));
		}
		// TODO: roll until reward depreciates please
		let reward_config = TaskRewardConfig::<Test>::get(task_id).unwrap();
		roll_to(reward_config.depreciation_rate.blocks * 3);
		assert_ok!(Tasks::submit_result(
			RawOrigin::Signed([0u8; 32].into()).into(),
			task_id,
			task_cycle,
			mock_result_ok(shard_id, task_id, task_cycle)
		));
		assert_eq!(<TaskState<Test>>::get(task_id), Some(TaskStatus::Completed));
		for (i, member) in shard_size_3().into_iter().enumerate() {
			assert_eq!(
				Balances::free_balance(&member) - balances[i],
				<Test as crate::Config>::BaseReadReward::get()
			);
		}
	});
}

// TODO:
// full reward up until first depreciation
// expected depreciation after first n blocks
// expected depreciation after first n*m blocks
// expected depreciation holds for multiple cycles
// TESTS for all rewards:
// - write
// - read
// - send_message
