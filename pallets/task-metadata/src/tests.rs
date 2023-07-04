use super::mock::*;
use crate::{mock, Error, Event};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::abstraction::{
	Collection, Function, Input, ObjectId, Output, PayableTask, Schema, Task, Validity,
};

#[test]
fn test_insert_task_metadata_task() {
	new_test_ext().execute_with(|| {
		let input: Task = Task {
			task_id: ObjectId(1),
			schema: vec![Schema::String(String::from("schema"))],
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
			network: time_primitives::sharding::Network::Ethereum,
			cycle: 12,
			with: vec![String::from("address")],
			validity: Validity::Seconds(10),
			hash: String::from("hash"),
		};

		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(1),
			1,
			Some(1),
			1,
			1
		));
		assert_ok!(TaskMeta::insert_task(RawOrigin::Signed(1).into(), input.clone(),));

		assert_eq!(TaskMeta::get_task_metadata(1), Some(input));
	});
}

#[test]
fn insert_task_should_fail_when_not_proxy_account() {
	new_test_ext().execute_with(|| {
		let input: Task = Task {
			task_id: ObjectId(1),
			schema: vec![Schema::String(String::from("schema"))],
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
			network: time_primitives::sharding::Network::Ethereum,
			cycle: 12,
			with: vec![String::from("address")],
			validity: Validity::Seconds(10),
			hash: String::from("hash"),
		};

		// Attempt to insert the payable task metadata without being a proxy account
		let result = TaskMeta::insert_task(RawOrigin::Signed(1).into(), input);

		// Assert that the function call did not succeed and produced the expected error
		assert_noop!(result, Error::<Test>::NotProxyAccount);
	});
}

#[test]
fn insert_task_check_event_duplicate_task_id() {
	new_test_ext().execute_with(|| {
		let input: Task = Task {
			task_id: ObjectId(1),
			schema: vec![Schema::String(String::from("schema"))],
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
			network: time_primitives::sharding::Network::Ethereum,
			cycle: 12,
			with: vec![String::from("address")],
			validity: Validity::Seconds(10),
			hash: String::from("hash"),
		};

		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(1),
			1,
			Some(1),
			1,
			1
		));

		// Insert the initial task with task ID 1
		assert_ok!(TaskMeta::insert_task(RawOrigin::Signed(1).into(), input));

		// Attempt to insert another task with the same task ID
		let duplicate_input: Task = Task {
			task_id: ObjectId(1), // Duplicate task ID
			schema: vec![Schema::String(String::from("schema"))],
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
			network: time_primitives::sharding::Network::Ethereum,
			cycle: 8, // Different cycle value for the duplicate task
			with: vec![String::from("address")],
			validity: Validity::Seconds(10),
			hash: String::from("hash"),
		};

		let result = TaskMeta::insert_task(RawOrigin::Signed(1).into(), duplicate_input);

		// Assert that the function call succeeded
		assert_ok!(result);

		// Assert that the `Event::AlreadyExist` event was emitted with the correct task ID
		assert!(System::events()
			.iter()
			.any(|event| { event.event == mock::RuntimeEvent::TaskMeta(Event::AlreadyExist(1)) }));
	});
}

#[test]
fn test_insert_collection_metadata() {
	new_test_ext().execute_with(|| {
		let input = Collection {
			hash: "collectionHash".to_string(),
			task: vec![1],
			validity: 1,
		};

		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(1),
			1,
			Some(1),
			1,
			1
		));

		assert_ok!(TaskMeta::insert_collection(
			RawOrigin::Signed(1).into(),
			input.hash.clone(),
			input.task.clone(),
			input.validity,
		));

		assert_eq!(TaskMeta::get_collection_metadata("collectionHash".to_string()), Some(input));
	});
}

#[test]
fn test_payable_task() {
	new_test_ext().execute_with(|| {
		//insert payable task metadata
		let input: PayableTask = PayableTask {
			task_id: ObjectId(1),
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
		};

		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(1),
			1,
			Some(1),
			1,
			1
		));
		assert_ok!(TaskMeta::insert_payable_task(RawOrigin::Signed(1).into(), input.clone(),));

		assert_eq!(TaskMeta::get_payable_task_metadata(1), Some(input));
	});
}

#[test]
fn insert_payable_task_should_fail_when_not_proxy_account() {
	new_test_ext().execute_with(|| {
		let input: PayableTask = PayableTask {
			task_id: ObjectId(1),
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
		};

		// Attempt to insert the payable task metadata without being a proxy account
		let result = TaskMeta::insert_payable_task(RawOrigin::Signed(1).into(), input);

		// Assert that the function call did not succeed and produced the expected error
		assert_noop!(result, Error::<Test>::NotProxyAccount);
	});
}

#[test]
fn insert_payable_task_check_event_duplicate_task_id() {
	new_test_ext().execute_with(|| {
		let input: PayableTask = PayableTask {
			task_id: ObjectId(1),
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
		};
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(1),
			1,
			Some(1),
			1,
			1
		));

		// Insert the initial payable task with task ID 1
		assert_ok!(TaskMeta::insert_payable_task(RawOrigin::Signed(1).into(), input));

		// Attempt to insert another task with the same task ID
		let duplicate_input: PayableTask = PayableTask {
			task_id: ObjectId(1),
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
		};

		let result = TaskMeta::insert_payable_task(RawOrigin::Signed(1).into(), duplicate_input);

		// Assert that the function call succeeded
		assert_ok!(result);

		// Assert that the `Event::PayableTaskMetaAlreadyExist` event was emitted with the correct task ID
		assert!(System::events().iter().any(|event| {
			event.event == mock::RuntimeEvent::TaskMeta(Event::PayableTaskMetaAlreadyExist(1))
		}));
	});
}
