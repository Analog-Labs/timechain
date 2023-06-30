use super::mock::*;
use frame_support::assert_ok;
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

		assert_ok!(TaskMeta::insert_collection(RawOrigin::Signed(1).into(),input.hash.clone(),input.task.clone(), input.validity.clone()));

		assert_eq!(
			TaskMeta::get_collection_metadata("collectionHash".to_string()),Some(input));
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
