use super::*;
use crate::Pallet;

use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;

use frame_system::RawOrigin;

benchmarks! {
	set_shard_config {
	}: _(RawOrigin::Root, 3, 1)
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
