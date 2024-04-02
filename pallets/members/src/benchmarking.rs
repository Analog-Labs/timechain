use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use time_primitives::{AccountId, NetworkId, PublicKey};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const ETHEREUM: NetworkId = 1;

fn public_key() -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(ALICE))
}

benchmarks! {
	register_member {
		let caller: AccountId = ALICE.into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(<T as Config>::MinStake::get() * 100),
		);
	}: _(RawOrigin::Signed(caller), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get())
	verify { }

	send_heartbeat {
		let caller: AccountId = ALICE.into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(<T as Config>::MinStake::get() * 100),
		);
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(caller.clone()).into(), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get());
	}: _(RawOrigin::Signed(caller), 0)
	verify { }

	unregister_member {
		let caller: AccountId = ALICE.into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(<T as Config>::MinStake::get() * 100),
		);
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(caller.clone()).into(), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get());
	}: _(RawOrigin::Signed(caller))
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
