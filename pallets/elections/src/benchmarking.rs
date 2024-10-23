use super::*;
use crate::Pallet;

use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::frame_system;

use frame_system::RawOrigin;
use time_primitives::{AccountId, NetworkId};

const ETHEREUM: NetworkId = 0;

benchmarks! {
	set_shard_config {
	}: _(RawOrigin::Root, 3, 1)
	verify { }

	try_elect_shard {
		let b in (ShardSize::<T>::get().into())..256;
		for i in 0..b {
			Unassigned::<T>::insert(ETHEREUM, Into::<AccountId>::into([i as u8; 32]), ());
		}
		let pre_unassigned_count: u16 = Unassigned::<T>::iter().count().try_into().unwrap_or_default();
	}: {
		Pallet::<T>::try_elect_shard(ETHEREUM);
	} verify {
		let post_unassigned_count: u16 = Unassigned::<T>::iter().count().try_into().unwrap_or_default();
		// ShardSize # of unassigned were elected to a shard
		assert_eq!(
			pre_unassigned_count - post_unassigned_count,
			ShardSize::<T>::get(),
		);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
