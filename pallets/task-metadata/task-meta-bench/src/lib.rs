#![cfg(feature = "runtime-benchmarks")]
#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused)]
#[cfg(test)]
mod mock;

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::{string::String, vec};
use task_metadata::{Call, Pallet as TaskMeta};
use time_primitives::abstraction::{Function, Input, ObjectId, Output, Schema, Task, Validity};

pub trait Config: task_metadata::Config + pallet_proxy::Config {}

pub struct Pallet<T: Config>(TaskMeta<T>);

benchmarks! {
	insert_task {
		let input: Task = Task {
			task_id: ObjectId(1),
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
		let proxy_acc: T::AccountId = whitelisted_caller();

		let _ = pallet_proxy::Pallet::<T>::set_proxy_account(RawOrigin::Signed(origin.clone()).into(), Some(10u32.into()), 10u32.into(), Some(10u32), 10u32, proxy_acc);
	}: _(RawOrigin::Signed(origin), task_data)
	verify {
		assert!(task_metadata::TaskMetaStorage::<T>::get(1).is_some());
	}

	insert_collection {
		let origin: T::AccountId = whitelisted_caller();
		let hash = String::from("hash");
		let	task = [0u8; 32];
		let validity = 10;
	}: _(RawOrigin::Signed(origin), hash.clone(), task.into(), validity)
	verify {
		assert!( task_metadata::CollectionMeta::<T>::get(hash).is_some());
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
