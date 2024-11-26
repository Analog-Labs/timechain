use super::*;
use crate::Pallet;

use polkadot_sdk::*;

use frame_benchmarking::benchmarks;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;
use time_primitives::{traits::IdentifyAccount, AccountId, NetworkId, PublicKey};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const ETHEREUM: NetworkId = 1;

fn public_key() -> PublicKey {
	pk_from_account(ALICE)
}

fn pk_from_account(r: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(r))
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
	}: _(RawOrigin::Signed(caller))
	verify { }

	unregister_member {
		let caller: AccountId = ALICE.into();
		pallet_balances::Pallet::<T>::resolve_creating(
			&caller,
			pallet_balances::Pallet::<T>::issue(<T as Config>::MinStake::get() * 100),
		);
		let _ = Pallet::<T>::register_member(RawOrigin::Signed(caller.clone()).into(), ETHEREUM, public_key(), ALICE, <T as Config>::MinStake::get());
	}: _(RawOrigin::Signed(caller), public_key().into_account())
	verify { }

	timeout_heartbeats {
		let b in 1..T::MaxTimeoutsPerBlock::get();
		for i in 0..b {
			let raw = [i as u8; 32];
			let caller: AccountId = raw.into();
			pallet_balances::Pallet::<T>::resolve_creating(
				&caller,
				pallet_balances::Pallet::<T>::issue(<T as Config>::MinStake::get() * 100),
			);
			Pallet::<T>::register_member(RawOrigin::Signed(caller.clone()).into(), ETHEREUM, pk_from_account(raw), caller.clone().into(), <T as Config>::MinStake::get())?;
			// Send heartbeat to set caller online and set heartbeat
			Pallet::<T>::send_heartbeat(RawOrigin::Signed(caller.clone()).into())?;
			assert!(MemberOnline::<T>::get(&caller).is_some());
			assert!(Heartbeat::<T>::get(&caller).is_some());
			// Add to timed out as if heartbeat was never submitted
			TimedOut::<T>::mutate(|x| x.push(caller.clone()));
		}
	}: {
		Pallet::<T>::timeout_heartbeats();
	} verify {
		for i in 0..b {
			let caller: AccountId = [i as u8; 32].into();
			assert!(MemberOnline::<T>::get(&caller).is_none());
			assert!(Heartbeat::<T>::get(&caller).is_none());
			// Next timed out set is derived from heartbeats previously in storage
			assert!(TimedOut::<T>::get().contains(&caller));
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
