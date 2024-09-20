use crate::mock::*;
use crate::ShardRegistered;

use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use polkadot_sdk::{frame_support, frame_system};
use time_primitives::{
	encode_gmp_events, BatchId, GatewayMessage, GatewayOp, GmpEvent, GmpMessage, GmpParams,
	NetworkId, ShardId, ShardStatus, ShardsInterface, Task, TaskId, TaskResult, TasksInterface,
};

const ETHEREUM: NetworkId = 0;

fn create_shard(network: NetworkId, n: u8, t: u16) -> ShardId {
	let mut members = vec![];
	for i in 0..n {
		members.push([i; 32].into());
	}
	let shard_id = Shards::create_shard(network, members, t).0;
	let pub_key = MockTssSigner::new().public_key();
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

fn submit_gateway_events(task_id: TaskId, events: Vec<GmpEvent>) {
	let bytes = encode_gmp_events(task_id, &events);
	let signature = MockTssSigner::new().sign(&bytes).to_bytes();
	let result = TaskResult::ReadGatewayEvents { events, signature };
	assert_ok!(Tasks::submit_task_result(
		RawOrigin::Signed([0; 32].into()).into(),
		task_id,
		result
	));
}

fn submit_message_signature(network: NetworkId, task: TaskId, batch: BatchId) {
	let msg = Tasks::get_batch_message(batch).unwrap();
	let bytes = msg.encode(batch);
	let hash = GmpParams { network, gateway: [0; 32] }.hash(&bytes);
	let signature = MockTssSigner::new().sign(&hash).to_bytes();
	let result = TaskResult::SignGatewayMessage { signature };
	assert_ok!(Tasks::submit_task_result(RawOrigin::Signed([0; 32].into()).into(), task, result));
}

fn submit_submission_error(task: TaskId, error: &str) {
	assert_ok!(Tasks::submit_task_result(
		RawOrigin::Signed([0; 32].into()).into(),
		task,
		TaskResult::SubmitGatewayMessage { error: error.to_string() }
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
		let shard_id = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard_id), vec![0]);
		submit_gateway_events(0, vec![]);
		assert_eq!(Tasks::get_task(2), Some(Task::ReadGatewayEvents { blocks: 47..52 }));
	})
}

#[test]
fn test_shard_online_registers_shard() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SignGatewayMessage { batch_id: 0 }));
		assert_eq!(
			Tasks::get_batch_message(0),
			Some(GatewayMessage {
				ops: vec![GatewayOp::RegisterShard(MockTssSigner::new().public_key())],
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
		assert_eq!(Tasks::get_task(2), Some(Task::SignGatewayMessage { batch_id: 1 }));
		assert_eq!(
			Tasks::get_batch_message(1),
			Some(GatewayMessage {
				ops: vec![GatewayOp::UnregisterShard(MockTssSigner::new().public_key())],
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
		submit_gateway_events(0, vec![GmpEvent::MessageReceived(msg.clone())]);
		roll(1);
		assert_eq!(Tasks::get_task(3), Some(Task::SignGatewayMessage { batch_id: 1 }));
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
fn test_sign_task_creates_submit_task() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SignGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 1);
		submit_message_signature(ETHEREUM, 1, 0);
		roll(1);
		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		assert!(Tasks::get_batch_signature(0).is_some());
	})
}

#[test]
fn test_shard_registered_event_registers_or_unregisters_shard() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(0), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		let shard_id = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_shard_tasks(shard_id), vec![0]);
		assert!(!Tasks::is_shard_registered(shard_id));
		submit_gateway_events(
			0,
			vec![GmpEvent::ShardRegistered(MockTssSigner::new().public_key())],
		);
		assert!(Tasks::is_shard_registered(shard_id));
		roll(1);
		submit_gateway_events(
			2,
			vec![GmpEvent::ShardUnregistered(MockTssSigner::new().public_key())],
		);
		assert!(!Tasks::is_shard_registered(shard_id));
	})
}

#[test]
fn test_msg_execution_event_completes_submit_task() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SignGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 1);
		submit_message_signature(ETHEREUM, 1, 0);
		roll(1);
		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 2);
		submit_gateway_events(0, vec![GmpEvent::BatchExecuted(0)]);
		assert_eq!(Tasks::get_task_result(2), Some(Ok(())));
	})
}

#[test]
fn test_msg_execution_error_completes_submit_task() {
	new_test_ext().execute_with(|| {
		register_gateway(ETHEREUM, 42);
		let shard = create_shard(ETHEREUM, 3, 1);
		roll(1);
		assert_eq!(Tasks::get_task(1), Some(Task::SignGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 1);
		submit_message_signature(ETHEREUM, 1, 0);
		roll(1);
		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 2);
		assert!(Tasks::get_task_result(2).is_none());
		submit_submission_error(2, "error message");
		assert_eq!(Tasks::get_task_result(2), Some(Err("error message".to_string())));
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
