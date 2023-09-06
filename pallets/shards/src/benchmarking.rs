use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use sp_std::vec;

pub const ALICE: [u8; 32] = [1u8; 32];

benchmarks! {
	commit {
	}: _(RawOrigin::Signed(ALICE.into()), 1, vec![], [0; 65])
	verify { }

	ready {
	}: _(RawOrigin::Signed(ALICE.into()), 1)
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
