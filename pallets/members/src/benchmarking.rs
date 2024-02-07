use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_support::traits::Get;
use frame_system::RawOrigin;
use time_primitives::{NetworkId, PublicKey};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const ETHEREUM: NetworkId = 1;

fn public_key() -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(ALICE))
}

benchmarks! {
	register_member {
	}: _(RawOrigin::Signed(ALICE.into()), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get())
	verify { }

	send_heartbeat {
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(ALICE.into()).into(), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get());
	}: _(RawOrigin::Signed(ALICE.into()))
	verify { }

	unregister_member {
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(ALICE.into()).into(), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get());
	}: _(RawOrigin::Signed(ALICE.into()))
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
