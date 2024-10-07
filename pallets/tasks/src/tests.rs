use crate::mock::*;
use crate::ShardRegistered;

use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use polkadot_sdk::{frame_support, frame_system};
use scale_codec::Encode;
use time_primitives::{
	traits::IdentifyAccount, ErrorMsg, GatewayMessage, GatewayOp, GmpEvent, GmpEvents, GmpMessage,
	MockTssSigner, NetworkId, PublicKey, ShardId, ShardStatus, ShardsInterface, Task, TaskId,
	TaskResult, TasksInterface, TssPublicKey, TssSignature,
};

const ETHEREUM: NetworkId = 0;

fn create_shard(network: NetworkId, n: u8, t: u16) -> ShardId {
	let mut members = vec![];
	for i in 0..n {
		members.push([i; 32].into());
	}
	let shard_id = Shards::create_shard(network, members, t).0;
	let pub_key = MockTssSigner::new(shard_id).public_key();
	ShardCommitment::<Test>::insert(shard_id, vec![pub_key]);
	ShardState::<Test>::insert(shard_id, ShardStatus::Online);
	Tasks::shard_online(shard_id, network);
	shard_id
}

fn shard_offline(network: NetworkId, shard: ShardId) {
	Tasks::shard_offline(shard, network);
}

fn register_gateway(network: NetworkId, block: u64) {
	Tasks::gateway_registered(network, block);
}

fn register_shard(shard: ShardId) {
	let public_key = Shards::tss_public_key(shard).unwrap();
	ShardRegistered::<Test>::insert(public_key, ());
}

fn submit_gateway_events(shard: ShardId, task_id: TaskId, events: &[GmpEvent]) {
	let signature = MockTssSigner::new(shard).sign_gmp_events(task_id, events);
	let result = TaskResult::ReadGatewayEvents {
		events: GmpEvents::truncate_from(events.to_vec()),
		signature,
	};
	assert_ok!(Tasks::submit_task_result(
		RawOrigin::Signed([0; 32].into()).into(),
		task_id,
		result
	));
}

fn submit_submission_error(account: PublicKey, task: TaskId, error: &str) {
	assert_ok!(Tasks::submit_task_result(
		RawOrigin::Signed(account.into_account()).into(),
		task,
		TaskResult::SubmitGatewayMessage {
			error: ErrorMsg::truncate_from(error.encode())
		}
	));
}

fn mock_gmp_msg(nonce: u64) -> GmpMessage {
	GmpMessage {
		src_network: ETHEREUM,
		dest_network: ETHEREUM,
		src: [0; 32],
		dest: [0; 32],
		nonce,
		gas_limit: 10_000,
		gas_cost: 10_000,
		bytes: vec![],
	}
}

#[test]
fn test_read_events_starts_when_gateway_is_registered() {
	new_test_ext().execute_with(|| {
		assert!(Tasks::get_task(0).is_none());
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		assert!(Tasks::get_task(1).is_none());
	})
}

#[test]
fn test_read_events_is_assigned_when_shard_is_online() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		let shard_id = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard_id), vec![0]);
	})
}

#[test]
fn test_read_events_completes_starts_next_read_events() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![0]);
		submit_gateway_events(shard, 0, &[]);
		assert_eq!(Tasks::get_task(2), Some(Task::ReadGatewayEvents { blocks: 47..52 }));
	})
}

#[test]
fn test_shard_online_registers_shard() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		assert_eq!(
			Tasks::get_batch_message(0),
			Some(GatewayMessage {
				ops: vec![GatewayOp::RegisterShard(MockTssSigner::new(shard).public_key())],
			})
		);
	})
}

#[test]
fn test_shard_offline_unregisters_shard() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		shard_offline(ETHEREUM, shard);
		roll(1);
		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 1 }));
		assert_eq!(
			Tasks::get_batch_message(1),
			Some(GatewayMessage {
				ops: vec![GatewayOp::UnregisterShard(MockTssSigner::new(shard).public_key())],
			})
		);
	})
}

#[test]
fn test_recv_msg_sends_msg() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![0]);
		let msg = mock_gmp_msg(0);
		submit_gateway_events(shard, 0, &[GmpEvent::MessageReceived(msg.clone())]);
		roll(1);
		assert_eq!(Tasks::get_task(3), Some(Task::SubmitGatewayMessage { batch_id: 1 }));
		assert_eq!(
			Tasks::get_batch_message(1),
			Some(GatewayMessage {
				ops: vec![GatewayOp::SendMessage(msg)],
			})
		);
	})
}

#[test]
fn test_shard_offline_unassigns_tasks() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![0]);
		shard_offline(ETHEREUM, shard);
		roll(1);
		assert!(Tasks::get_shard_tasks(shard).is_empty());
	})
}

#[test]
fn test_shard_registered_event_registers_or_unregisters_shard() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![0]);
		assert!(!Tasks::is_shard_registered(shard));
		submit_gateway_events(
			shard,
			0,
			&[GmpEvent::ShardRegistered(MockTssSigner::new(shard).public_key())],
		);
		assert!(Tasks::is_shard_registered(shard));
		roll(1);
		submit_gateway_events(
			shard,
			2,
			&[GmpEvent::ShardUnregistered(MockTssSigner::new(shard).public_key())],
		);
		assert!(!Tasks::is_shard_registered(shard));
	})
}

#[test]
fn test_msg_execution_event_completes_submit_task() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 1);
		submit_gateway_events(shard, 0, &[GmpEvent::BatchExecuted(0)]);
		assert_eq!(Tasks::get_task_result(1), Some(Ok(())));
	})
}

#[test]
fn test_msg_execution_error_completes_submit_task() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 1);
		assert!(Tasks::get_task_result(1).is_none());
		let account = Tasks::get_task_submitter(1).unwrap();
		submit_submission_error(account, 1, "error message");
		assert_eq!(
			Tasks::get_task_result(1),
			Some(Err(ErrorMsg::truncate_from("error message".encode())))
		);
	})
}

#[test]
fn test_tasks_are_assigned_to_registered_shards() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		register_shard(shard);
		assert!(Tasks::is_shard_registered(shard));
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![1, 0]);
	})
}

#[test]
fn test_max_tasks_per_block() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		register_shard(shard);
		assert!(Tasks::is_shard_registered(shard));
		Tasks::create_task(ETHEREUM, Task::SubmitGatewayMessage { batch_id: 0 });
		Tasks::create_task(ETHEREUM, Task::SubmitGatewayMessage { batch_id: 1 });
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![1, 0, 2]);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard), vec![3, 1, 0, 2]);
	})
}

mod bench_helper {
	use super::*;

	fn valid_pk_task_result() -> (TssPublicKey, TssSignature) {
		let signer = MockTssSigner::new(SHARD_ID);
		const TASK_ID: TaskId = 0;
		const SHARD_ID: ShardId = 0;
		let signature = signer.sign_gmp_events(TASK_ID, &[]);
		(signer.public_key(), signature)
	}

	#[test]
	#[ignore]
	fn print_valid_result() {
		let (signer, signature) = valid_pk_task_result();
		println!("signer: {:?}\nsignature: {:?}", signer, signature);
		panic!();
	}
}
