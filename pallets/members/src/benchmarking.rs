use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use time_primitives::{Network, PublicKey};

pub const ALICE: [u8; 32] = [1u8; 32];

fn public_key() -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(ALICE))
}

benchmarks! {
	register_member {
	}: _(RawOrigin::Signed(ALICE.into()), Network::Ethereum, public_key(), ALICE)
	verify { }

	send_heartbeat {
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(ALICE.into()).into(), Network::Ethereum, public_key(), ALICE);
	}: _(RawOrigin::Signed(ALICE.into()))
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
