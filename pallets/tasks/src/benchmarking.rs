use crate::{Call, Config, Pallet};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::traits::Currency;
use frame_system::RawOrigin;
use pallet_shards::ShardState;
use scale_info::prelude::vec;
use time_primitives::{
	Function, NetworkId, Payload, ShardStatus, ShardsInterface, TaskDescriptorParams, TaskResult,
	TasksInterface,
};

const ETHEREUM: NetworkId = 1;

benchmarks! {
	where_clause {  where T: pallet_shards::Config }
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
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller = whitelisted_caller();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(10_000),
		);
	}: _(RawOrigin::Signed(whitelisted_caller()), descriptor) verify {}

	submit_result {
		let descriptor = TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmViewCall {
				address: Default::default(),
				input: Default::default(),
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		};
		<T as Config>::Shards::create_shard(
			ETHEREUM,
			[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
			1,
		);
		ShardState::<T>::insert(0, ShardStatus::Online);
		Pallet::<T>::shard_online(0, ETHEREUM);
		let caller = whitelisted_caller();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(10_000),
		);
		Pallet::<T>::create_task(RawOrigin::Signed(caller.clone()).into(), descriptor)?;
		let result = TaskResult {
			shard_id: 0,
			payload: Payload::Hashed([0; 32]),
			signature: [0; 64],
		};
	}: _(RawOrigin::Signed(caller), 0, result) verify {}

	submit_hash {
		let b in 1..10000;
		Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::EvmCall {
				address: Default::default(),
				input: Default::default(),
				amount: 0,
				gas_limit: None,
			},
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		})?;
		Pallet::<T>::shard_online(1, ETHEREUM);
	}: _(RawOrigin::Signed(whitelisted_caller()), 1, [b as _; 32]) verify {}

	submit_signature {
		Pallet::<T>::create_task(RawOrigin::Signed(whitelisted_caller()).into(), TaskDescriptorParams {
			network: ETHEREUM,
			function: Function::SendMessage { msg: Default::default() },
			start: 0,
			funds: 100u32.into(),
			shard_size: 3,
		})?;
		Pallet::<T>::shard_online(1, ETHEREUM);
	}: _(RawOrigin::Signed(whitelisted_caller()), 0, [0u8; 64]) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
