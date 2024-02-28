use crate::{Call, Config, Pallet};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use scale_info::prelude::vec;
use time_primitives::{
	Function, NetworkId, Payload, TaskDescriptorParams, TaskResult, TasksInterface,
};

const ETHEREUM: NetworkId = 1;

benchmarks! {
	create_task {
		let b in 1..10000;
		let input = vec![0u8; b as usize];
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input,
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		};
	}: _(RawOrigin::Signed(whitelisted_caller()), descriptor) verify {}

	submit_result {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input: Default::default(),
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		});
		Pallet::<T>::shard_online(1, ETHEREUM);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0, TaskResult {
		shard_id: 1,
		payload: Payload::Hashed([0; 32]),
		signature: [0; 64],
	}) verify {}

	submit_hash {
		let b in 1..10000;
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmCall {
				address: Default::default(),
				input: Default::default(),
				amount: 0,
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		});
		Pallet::<T>::shard_online(1, ETHEREUM);
	}: _(RawOrigin::Signed(whitelisted_caller()), 1, [b as _; 32]) verify {}

	submit_signature {
		let _ = Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::SendMessage { msg: Default::default() },
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		});
		Pallet::<T>::shard_online(1, ETHEREUM);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0, [0u8; 64], 0) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
