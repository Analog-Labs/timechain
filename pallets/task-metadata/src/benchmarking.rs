use super::*;
#[allow(unused)]
use crate::Pallet as TaskMeta;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::{string::String, vec};
use time_primitives::abstraction::{Function, Input, ObjectId, Output, Schema, Task, Validity};

benchmarks! {
	insert_task {
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

		let origin: T::AccountId = whitelisted_caller();
		let task_data = input.clone();
	}: _(RawOrigin::Signed(origin), task_data)
	verify {
		assert!( <TaskMetaStorage<T>>::get(1).is_some());
	}

	insert_collection {
		let origin: T::AccountId = whitelisted_caller();
		let hash = String::from("hash");
		let	task = [0u8; 32];
		let validity = 10;
	}: _(RawOrigin::Signed(origin), hash.clone(), task.into(), validity)
	verify {
		assert!( <CollectionMeta<T>>::get(hash).is_some());
	}

	impl_benchmark_test_suite!(TaskMeta, crate::mock::new_test_ext(), crate::mock::Test);
}
