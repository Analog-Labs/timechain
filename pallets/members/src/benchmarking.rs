use super::*;
use crate::Pallet;

use polkadot_sdk::*;

use frame_benchmarking::benchmarks;
use frame_support::traits::Get;
use frame_system::RawOrigin;
use time_primitives::{AccountId, MembersInterface, NetworkId};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const ETHEREUM: NetworkId = 1;

benchmarks! {
	register_member {
	}: _(RawOrigin::Root, ETHEREUM, ALICE.into(), ALICE)
	verify { }

	send_heartbeat {
		let _ = Pallet::<T>::register_member(RawOrigin::Root.into(), ETHEREUM, ALICE.into(), ALICE);
	}: _(RawOrigin::Signed(ALICE.into()))
	verify { }

	unregister_member {
		let _ = Pallet::<T>::register_member(RawOrigin::Root.into(), ETHEREUM, ALICE.into(), ALICE);
	}: _(RawOrigin::Root, ALICE.into())
	verify { }

	timeout_heartbeats {
		let b in 1..T::MaxTimeoutsPerBlock::get();
		for i in 0..b {
			let raw = [i as u8; 32];
			let acc: AccountId = raw.into();
			Pallet::<T>::register_member(RawOrigin::Root.into(), ETHEREUM, acc.clone(), raw)?;
			// Send heartbeat to set caller online and set heartbeat
			Pallet::<T>::send_heartbeat(RawOrigin::Signed(acc.clone()).into())?;
			assert!(MemberOnline::<T>::get(&acc).is_some());
			assert!(Heartbeat::<T>::get(&acc).is_some());
			// Add to timed out as if heartbeat was never submitted
			TimedOut::<T>::mutate(|x| x.push(acc.clone()));
		}
	}: {
		Pallet::<T>::timeout_heartbeats();
	} verify {
		for i in 0..b {
			let raw = [i as u8; 32];
			let acc: AccountId = raw.into();
			assert!(MemberOnline::<T>::get(&acc).is_none());
			assert!(Heartbeat::<T>::get(&acc).is_none());
			// Next timed out set is derived from heartbeats previously in storage
			assert!(TimedOut::<T>::get().contains(&acc));
		}
	}

	is_member {
		let _ = Pallet::<T>::register_member(RawOrigin::Root.into(), ETHEREUM, ALICE.into(), ALICE);

		let result: bool;
	} : {
		result = Pallet::<T>::is_member_registered(&ALICE.into());
	} verify {
		assert!(result);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
