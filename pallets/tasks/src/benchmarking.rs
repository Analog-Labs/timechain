use crate::{Call, Config, Pallet};
use frame_benchmarking::benchmarks;
use frame_support::pallet_prelude::Get;
use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;
use pallet_networks::NetworkGatewayAddress;
use pallet_shards::{ShardCommitment, ShardState};
use polkadot_sdk::{frame_benchmarking, frame_support, frame_system, sp_std};
use sp_std::vec;
use time_primitives::{
	NetworkId, ShardStatus, ShardsInterface, Task, TaskResult, TasksInterface, TssPublicKey,
	TssSignature,
};

// Generated by running tests::bench_helper::print_valid_result
const PUBKEY: TssPublicKey = [
	2, 121, 190, 102, 126, 249, 220, 187, 172, 85, 160, 98, 149, 206, 135, 11, 7, 2, 155, 252, 219,
	45, 206, 40, 217, 89, 242, 129, 91, 22, 248, 23, 152,
];
const SIGNATURE: TssSignature = [
	119, 119, 90, 235, 7, 120, 43, 92, 165, 239, 64, 155, 193, 120, 121, 158, 221, 101, 67, 71, 77,
	30, 153, 33, 133, 21, 151, 152, 72, 54, 125, 208, 207, 130, 157, 92, 45, 249, 30, 88, 128, 186,
	115, 60, 151, 220, 75, 88, 253, 91, 155, 144, 186, 107, 101, 60, 63, 169, 178, 192, 80, 234,
	63, 100,
];

fn create_simple_task<T: Config + pallet_shards::Config>() {
	const ETHEREUM: NetworkId = 0;
	let (shard_id, _) = <T as Config>::Shards::create_shard(
		ETHEREUM,
		[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
		1,
	);
	ShardState::<T>::insert(shard_id, ShardStatus::Online);
	ShardCommitment::<T>::insert(shard_id, vec![PUBKEY]);
	Pallet::<T>::shard_online(shard_id, ETHEREUM);
	Pallet::<T>::create_task(ETHEREUM, Task::ReadGatewayEvents { blocks: 0..10 });
}

benchmarks! {
	where_clause {  where T: pallet_shards::Config + pallet_members::Config + pallet_networks::Config }

	submit_task_result {
		NetworkGatewayAddress::<T>::insert(0, [0; 32]);
		create_simple_task::<T>();
		Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		let result = TaskResult::ReadGatewayEvents { events: vec![], signature: SIGNATURE };
	}: _(RawOrigin::Signed([0u8; 32].into()), 0, result) verify {}

	schedule_tasks {
		let b in 1..<T as Config>::MaxTasksPerBlock::get();
		for i in 0..b {
			create_simple_task::<T>();
		}
		Pallet::<T>::prepare_batches();
	}: {
		Pallet::<T>::schedule_tasks();
	} verify { }

	prepare_batches {
		let b in 1..<T as Config>::MaxBatchesPerBlock::get();
		for i in 0..b {
			create_simple_task::<T>();
		}
	}: {
		Pallet::<T>::prepare_batches();
	} verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
