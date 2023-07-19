#![cfg(feature = "runtime-benchmarks")]
#![cfg_attr(not(feature = "std"), no_std)]

#[allow(unused)]
#[cfg(test)]
mod mock;

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::string::String;
use task_schedule::{Call, Pallet as TaskSchedule};
use time_primitives::abstraction::{
	ObjectId, ScheduleInput as Schedule, ScheduleStatus, TaskSchedule as TaskScheduleInput,
	Validity,
};

pub trait Config: task_schedule::Config + pallet_proxy::Config {}

pub struct Pallet<T: Config>(TaskSchedule<T>);

benchmarks! {

	insert_schedule {
		let origin: T::AccountId = whitelisted_caller();
		let input  = Schedule {
			task_id: ObjectId(1),
			shard_id: 1,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash:String::from("address"),
		};

		let schedule = input.clone();
		let proxy_acc: T::AccountId = whitelisted_caller();

		let _ = pallet_proxy::Pallet::<T>::set_proxy_account(RawOrigin::Signed(origin.clone()).into(), Some(10u32.into()), 10u32.into(), Some(10u32), 10u32, proxy_acc);

	}: _(RawOrigin::Signed(origin), schedule)
	verify {
		assert!( task_schedule::ScheduleStorage::<T>::get(1).is_some());
	}

	update_schedule {
		let origin: T::AccountId = whitelisted_caller();
		let input  = TaskScheduleInput {
			task_id: ObjectId(1),
			owner:origin.clone(),
			shard_id: 1,
			cycle: 12,
			start_block: 0,
			validity: Validity::Seconds(10),
			hash:String::from("address"),
			status: ScheduleStatus::Initiated,
		};
		let proxy_acc: T::AccountId = whitelisted_caller();

		let _ = pallet_proxy::Pallet::<T>::set_proxy_account(RawOrigin::Signed(origin.clone()).into(), Some(10u32.into()), 10u32.into(), Some(10u32), 10u32, proxy_acc);

		task_schedule::ScheduleStorage::<T>::insert(
			1,
			input
		);
	}: _(RawOrigin::Signed(origin), ScheduleStatus::Completed, 1)
	verify {
		assert!( task_schedule::ScheduleStorage::<T>::get(1).is_some());
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
