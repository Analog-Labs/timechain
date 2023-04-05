use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{
	abstraction::{Function, Input, ObjectId, Output, Schema, Task, Validity}
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
				output:vec![Output::Skip],
			},
			cycle: 12,
			with: vec![String::from("address")],
			validity: Validity::Seconds(10),
			hash: String::from("hash"),
		};
		assert_ok!(TaskMeta::insert_task(RawOrigin::Signed(1).into(), input.clone(),));

		assert_eq!(TaskMeta::get_task_metadata(1), Some(input));
	});
}
