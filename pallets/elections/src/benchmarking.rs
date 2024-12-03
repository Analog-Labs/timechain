use super::*;
use crate::Pallet;

use frame_system::RawOrigin;
use pallet_members::{MemberOnline, MemberStake};
use pallet_networks::NetworkName;
use polkadot_sdk::frame_benchmarking::benchmarks;
use polkadot_sdk::{frame_support::traits::Get, frame_system, sp_runtime};
use scale_codec::Encode;
use sp_runtime::{BoundedVec, Vec};
use time_primitives::{AccountId, ChainName, ChainNetwork, ElectionsInterface, NetworkId};
const ETHEREUM: NetworkId = 0;

benchmarks! {
	where_clause { where T: pallet_members::Config + pallet_networks::Config + pallet_shards::Config }

	set_shard_config {
	}: _(RawOrigin::Root, 3, 1)
	verify { }

	try_elect_shards {
		let b in 1..<T as pallet_shards::Config>::MaxTimeoutsPerBlock::get();
		// Insert network
		let net_name = (
			ChainName(BoundedVec::truncate_from("ETHEREUM".encode())),
			ChainNetwork(BoundedVec::truncate_from("SEPOLIA".encode())),
		);
		NetworkName::<T>::insert(ETHEREUM, net_name);
		// Register enough members for `b` new shards
		let mut all_new_shard_members = Vec::new();
		let account = |x, y| -> AccountId {
			let mut acc = [0u8; 32];
			acc[..16].copy_from_slice(&[x as u8; 16]);
			acc[16..].copy_from_slice(&[y as u8; 16]);
			Into::<AccountId>::into(acc)
		};
		for i in 0..b {
			for j in 0..ShardSize::<T>::get() {
				let member = account(i, j);
				MemberOnline::<T>::insert(member.clone(), ());
				Pallet::<T>::member_online(&member, ETHEREUM);
				let member_stake: u128 = 1_000_000_000 - <u32 as Into<u128>>::into(i) -  <u16 as Into<u128>>::into(j);
				MemberStake::<T>::insert(member.clone(), member_stake);
				all_new_shard_members.push(member);
			}
		}
		let pre_unassigned_count: u16 = Unassigned::<T>::get(ETHEREUM).len() as u16;
	}: {
		Pallet::<T>::try_elect_shards(ETHEREUM, b);
	} verify {
		let post_unassigned_count: u16 = Unassigned::<T>::get(ETHEREUM).len() as u16;
		// ShardSize # of unassigned were elected to a shard
		assert_eq!(
			pre_unassigned_count - post_unassigned_count,
			(b as u16).saturating_mul(ShardSize::<T>::get()),
		);
		let unassigned = Unassigned::<T>::get(ETHEREUM);
		// New shard members were removed from Unassigned
		for m in all_new_shard_members {
			assert!(!unassigned.contains(&m));
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
