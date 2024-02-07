use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;

benchmarks! {
	add_network {
	}: _(RawOrigin::Root, "ethereum".into(), "dev".into())
	verify {}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
