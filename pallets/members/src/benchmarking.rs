use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use time_primitives::Network;

pub const ALICE: [u8; 32] = [1u8; 32];

benchmarks! {
	register_member {
	}: _(RawOrigin::Signed(ALICE.into()), Network::Ethereum, ALICE)
	verify { }

	send_heartbeat {
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(ALICE.into()).into(), Network::Ethereum, ALICE);
	}: _(RawOrigin::Signed(ALICE.into()))
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
