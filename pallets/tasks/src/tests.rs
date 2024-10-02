use crate::mock::*;
use crate::ShardRegistered;

use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use polkadot_sdk::{frame_support, frame_system};
use time_primitives::{
	traits::IdentifyAccount, BatchId, GatewayMessage, GatewayOp, GmpEvent, GmpMessage,
	MockTssSigner, Network, NetworkConfig, NetworkId, PublicKey, ShardId, ShardStatus,
	ShardsInterface, Task, TaskId, TaskResult, TasksInterface, TssPublicKey, TssSignature,
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
	let _ = Networks::register_network(
		frame_system::RawOrigin::Root.into(),
		Network {
			id: network,
			chain_name: "Ethereum".into(),
			chain_network: "Mainnet".into(),
			gateway: [0; 32],
			gateway_block: block,
			config: NetworkConfig {
				batch_size: 5,
				batch_offset: 0,
				batch_gas_limit: 10_000,
				shard_task_limit: 10,
			},
		},
	);
	Tasks::gateway_registered(network, block);
}

fn register_shard(shard: ShardId) {
	let public_key = Shards::tss_public_key(shard).unwrap();
	ShardRegistered::<Test>::insert(public_key, ());
}

fn submit_gateway_events(shard: ShardId, task_id: TaskId, events: &[GmpEvent]) {
	let signature = MockTssSigner::new(shard).sign_gmp_events(task_id, events);
	let result = TaskResult::ReadGatewayEvents {
		events: events.to_vec(),
		signature,
	};
	assert_ok!(Tasks::submit_task_result(
		RawOrigin::Signed([0; 32].into()).into(),
		task_id,
		result
	));
}

fn submit_message_signature(shard: ShardId, task: TaskId, batch: BatchId) {
	let network = Shards::shard_network(shard).unwrap();
	let msg = Tasks::get_batch_message(batch).unwrap();
	let signature = MockTssSigner::new(shard).sign_gateway_message(network, [0; 32], batch, &msg);
	let result = TaskResult::SignGatewayMessage { signature };
	assert_ok!(Tasks::submit_task_result(RawOrigin::Signed([0; 32].into()).into(), task, result));
}

fn submit_submission_error(account: PublicKey, task: TaskId, error: &str) {
	assert_ok!(Tasks::submit_task_result(
		RawOrigin::Signed(account.into_account()).into(),
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
		assert_eq!(Tasks::get_task(1), Some(Task::SignGatewayMessage { batch_id: 0 }));
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
		assert_eq!(Tasks::get_task(2), Some(Task::SignGatewayMessage { batch_id: 1 }));
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
		submit_message_signature(shard, 1, 0);
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
		assert_eq!(Tasks::get_task(1), Some(Task::SignGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 1);
		submit_message_signature(shard, 1, 0);
		roll(1);
		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 2);
		submit_gateway_events(shard, 0, &[GmpEvent::BatchExecuted(0)]);
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
		submit_message_signature(shard, 1, 0);
		roll(1);
		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
		Tasks::assign_task(shard, 2);
		assert!(Tasks::get_task_result(2).is_none());
		let account = Tasks::get_task_submitter(2).unwrap();
		submit_submission_error(account, 2, "error message");
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
