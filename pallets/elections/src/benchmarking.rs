use super::*;
use crate::Pallet;

use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;

use frame_system::RawOrigin;
use time_primitives::{AccountId, ElectionsInterface, NetworkId};

const ETHEREUM: NetworkId = 0;

benchmarks! {
	set_shard_config {
	}: _(RawOrigin::Root, 3, 1)
	verify { }

	try_elect_shard {
		let b in 1..256;
		for i in 0..b {
			Unassigned::<T>::insert(ETHEREUM, Into::<AccountId>::into([i as u8; 32]), ());
		}
	}: {
		Pallet::<T>::try_elect_shard(ETHEREUM);
	} verify { }

	member_offline {
		let member: AccountId = [0u8; 32].into();
		Unassigned::<T>::insert(ETHEREUM, member.clone(), ());
		assert!(Unassigned::<T>::get(ETHEREUM, member.clone()).is_some());
	}: {
		Pallet::<T>::member_offline(&member, ETHEREUM);
	} verify {
		assert!(Unassigned::<T>::get(ETHEREUM, member).is_none());
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
