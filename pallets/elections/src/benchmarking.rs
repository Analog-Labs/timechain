use super::*;
use crate::Pallet;

use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::{frame_system, sp_runtime};

use frame_system::RawOrigin;
use pallet_members::{MemberOnline, MemberStake};
use sp_runtime::Vec;
use time_primitives::{AccountId, NetworkId};
const ETHEREUM: NetworkId = 0;

benchmarks! {
	where_clause { where T: pallet_members::Config }

	set_shard_config {
	}: _(RawOrigin::Root, 3, 1)
	verify { }

	try_elect_shard {
		let b in (ShardSize::<T>::get().into())..256;
		let max_stake: u128 = 1_000_000_000;
		let mut shard: Vec<AccountId> = Vec::new();
		for i in 0..b {
			let member = Into::<AccountId>::into([i as u8; 32]);
			Unassigned::<T>::insert(ETHEREUM, member.clone(), ());
			MemberOnline::<T>::insert(member.clone(), ());
			let member_stake: u128 = max_stake - Into::<u128>::into(i);
			MemberStake::<T>::insert(member.clone(), member_stake);
			if (shard.len() as u16) < ShardSize::<T>::get() {
				shard.push(member);
			}
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
		// New shard members were removed from Unassigned
		for m in shard {
			assert!(Unassigned::<T>::get(ETHEREUM, m).is_none());
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
