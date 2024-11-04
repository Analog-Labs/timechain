use crate::{mock::*, Error, Event, ShardSize, ShardThreshold, Unassigned};

use polkadot_sdk::*;

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use pallet_members::MemberOnline;
use time_primitives::{ElectionsInterface, NetworkId, MAX_SHARD_SIZE};

const ETHEREUM: NetworkId = 0;

#[test]
fn new_member_online_inserts_unassigned() {
	let a: AccountId = [1u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, ETHEREUM);
		assert!(Unassigned::<Test>::get(ETHEREUM, &a).is_some());
	});
}

#[test]
fn shard_size_new_members_online_creates_shard() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, ETHEREUM);
		MemberOnline::<Test>::insert(&a, ());
		roll(1);
		assert!(Unassigned::<Test>::get(ETHEREUM, &a).is_some());
		Elections::member_online(&b, ETHEREUM);
		MemberOnline::<Test>::insert(&b, ());
		roll(1);
		assert!(Unassigned::<Test>::get(ETHEREUM, &b).is_some());
		Elections::member_online(&c, ETHEREUM);
		MemberOnline::<Test>::insert(&c, ());
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(0, ETHEREUM).into());
		for member in [a, b, c] {
			assert!(Unassigned::<Test>::get(ETHEREUM, member).is_none());
		}
		assert_eq!(pallet_shards::ShardThreshold::<Test>::get(0), Some(2));
	});
}

#[test]
fn member_offline_removes_unassigned() {
	let a: AccountId = [1u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, ETHEREUM);
		assert!(Unassigned::<Test>::get(ETHEREUM, &a).is_some());
		Elections::member_offline(&a, ETHEREUM);
		assert!(Unassigned::<Test>::get(ETHEREUM, &a).is_none());
	});
}

fn register_member(pubkey: [u8; 32], stake: u128) {
	let staker = pubkey.into();
	assert_ok!(Members::register_member(
		RawOrigin::Signed(staker).into(),
		ETHEREUM,
		pubkey_from_bytes(pubkey),
		pubkey,
		stake,
	));
}

#[test]
fn shard_offline_automatically_creates_new_shard() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	new_test_ext().execute_with(|| {
		register_member([1u8; 32], 5);
		Elections::member_online(&a, ETHEREUM);
		MemberOnline::<Test>::insert(&a, ());
		register_member([2u8; 32], 5);
		Elections::member_online(&b, ETHEREUM);
		MemberOnline::<Test>::insert(&b, ());
		register_member([3u8; 32], 5);
		Elections::member_online(&c, ETHEREUM);
		MemberOnline::<Test>::insert(&c, ());
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(0, ETHEREUM).into());
		Elections::shard_offline(ETHEREUM, [a, b, c].to_vec());
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(1, ETHEREUM).into());
	});
}

#[test]
fn set_shard_config_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(ShardSize::<Test>::get(), 3);
		assert_eq!(ShardThreshold::<Test>::get(), 2);
		assert_ok!(Elections::set_shard_config(RawOrigin::Root.into(), 2, 1));
		System::assert_last_event(Event::<Test>::ShardConfigSet(2, 1).into());
		assert_eq!(ShardSize::<Test>::get(), 2);
		assert_eq!(ShardThreshold::<Test>::get(), 1);
		assert_noop!(
			Elections::set_shard_config(RawOrigin::Root.into(), 1, 2),
			Error::<Test>::ThresholdLargerThanSize
		);
		let max_shard_size_plus_one: u16 =
			1u16.saturating_add(MAX_SHARD_SIZE.try_into().unwrap_or_default());
		assert_noop!(
			Elections::set_shard_config(RawOrigin::Root.into(), max_shard_size_plus_one, 1),
			Error::<Test>::ShardSizeAboveMax
		);
	});
}
