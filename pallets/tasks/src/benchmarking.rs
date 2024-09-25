use crate::{Call, Config, Pallet};
use frame_benchmarking::benchmarks;
use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;
use pallet_shards::{ShardCommitment, ShardState};
use polkadot_sdk::{frame_benchmarking, frame_support, frame_system, sp_core, sp_std};
use sp_std::vec;
use time_primitives::{
	NetworkId, PublicKey, ShardStatus, ShardsInterface, Task, TaskResult, TasksInterface,
	TssPublicKey,
};

const ETHEREUM: NetworkId = 0;
const PUBKEY: TssPublicKey = [
	2, 36, 79, 43, 160, 29, 26, 4, 168, 242, 35, 104, 66, 1, 179, 183, 189, 197, 92, 84, 2, 101,
	52, 245, 230, 250, 199, 131, 188, 204, 228, 70, 248,
];

fn create_simple_task<T: Config + pallet_shards::Config>() {
	<T as Config>::Shards::create_shard(
		ETHEREUM,
		[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()].to_vec(),
		1,
	);
	ShardState::<T>::insert(0, ShardStatus::Online);
	ShardCommitment::<T>::insert(0, vec![PUBKEY]);
	Pallet::<T>::shard_online(0, ETHEREUM);
	Pallet::<T>::create_task(ETHEREUM, Task::ReadGatewayEvents { blocks: 0..10 });
	Pallet::<T>::assign_task(0, 0);
	Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
}

benchmarks! {
	where_clause {  where T: pallet_shards::Config + pallet_members::Config }

	submit_task_result {
		create_simple_task::<T>();
		let result = TaskResult::ReadGatewayEvents { events: vec![], signature: [0u8; 64] };
	}: _(RawOrigin::Signed([0u8; 32].into()), 0, result) verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
