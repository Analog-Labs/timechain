use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{
	abstraction::{Function, Input, ObjectId, Output, PayableTask, Schema, Task, Validity},
	ProxyAccInput,
};

#[test]
fn test_task() {
	new_test_ext().execute_with(|| {
		let input: Task = Task {
			collection_id: ObjectId(1),
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

		let proxy_data = ProxyAccInput {
			proxy: 1,
			max_token_usage: Some(1),
			token_usage: 1,
			max_task_execution: Some(1),
			task_executed: 1,
		};

		let _ = PalletProxy::set_proxy_account(RawOrigin::Signed(1).into(), proxy_data);
		assert_ok!(TaskMeta::insert_task(RawOrigin::Signed(1).into(), input.clone(),));

		assert_eq!(TaskMeta::get_task_metadata(1), Some(input));
	});
}

#[test]
fn test_payable_task() {
	new_test_ext().execute_with(|| {
		//insert payable task metadata
		let input: PayableTask = PayableTask {
			collection_id: ObjectId(1),
			function: Function::EthereumApi {
				function: String::from("function name"),
				input: vec![Input::HexAddress],
				output: vec![Output::Skip],
			},
		};

		let proxy_data = ProxyAccInput {
			proxy: 1,
			max_token_usage: Some(1),
			token_usage: 1,
			max_task_execution: Some(1),
			task_executed: 1,
		};

		let _ = PalletProxy::set_proxy_account(RawOrigin::Signed(1).into(), proxy_data);
		assert_ok!(TaskMeta::insert_payable_task(RawOrigin::Signed(1).into(), input.clone(),));

		assert_eq!(TaskMeta::get_payable_task_metadata(1), Some(input));
	});
}
