use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;

benchmarks! {
	submit_tss_public_key {
	}: _(RawOrigin::None, 1, [0; 33])
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
