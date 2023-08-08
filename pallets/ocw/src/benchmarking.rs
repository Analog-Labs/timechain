use crate::{Call, Config, Pallet};
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use time_primitives::{PublicKey, ScheduleStatus, ShardCreated, ShardId, TssPublicKey};

fn collector() -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw([42; 32]))
}

const SHARD_ID: ShardId = 42;
const TSS_PUBLIC_KEY: TssPublicKey = [42; 33];

benchmarks! {
	submit_tss_public_key {
		Pallet::<T>::shard_created(SHARD_ID, collector());
	}: _(RawOrigin::Signed([42; 32].into()), SHARD_ID, TSS_PUBLIC_KEY)
	verify { }

	submit_task_result {
		Pallet::<T>::shard_created(SHARD_ID, collector());
	}: _(RawOrigin::Signed([42; 32].into()), 0, 1, ScheduleStatus {
		shard_id: SHARD_ID, result: Ok([0; 64])
	}) verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
