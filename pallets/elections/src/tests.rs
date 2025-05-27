use crate::{mock::*, Unassigned};

use polkadot_sdk::*;

use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_members::MemberOnline;
use time_primitives::{ElectionsInterface, NetworkId};

const ETHEREUM: NetworkId = 0;

#[test]
fn new_member_online_inserts_unassigned() {
	let a: AccountId = [1u8; 32].into();
	new_test_ext().execute_with(|| {
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		assert!(Unassigned::<Test>::get(ETHEREUM).contains(&a));
	});
}

#[test]
fn shard_size_new_members_online_creates_shard() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	new_test_ext().execute_with(|| {
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		roll(1);
		assert!(Unassigned::<Test>::get(ETHEREUM).contains(&a));
		MemberOnline::<Test>::insert(&b, ());
		Elections::member_online(&b, ETHEREUM);
		roll(1);
		assert!(Unassigned::<Test>::get(ETHEREUM).contains(&b));
		MemberOnline::<Test>::insert(&c, ());
		Elections::member_online(&c, ETHEREUM);
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(0, ETHEREUM).into());
		for member in [a, b, c] {
			assert!(!Unassigned::<Test>::get(ETHEREUM).contains(&member));
		}
	});
}

#[test]
fn member_offline_removes_unassigned() {
	let a: AccountId = [1u8; 32].into();
	new_test_ext().execute_with(|| {
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		assert!(Unassigned::<Test>::get(ETHEREUM).contains(&a));
		Elections::members_offline(vec![a.clone()], ETHEREUM);
		assert!(!Unassigned::<Test>::get(ETHEREUM).contains(&a));
	});
}

fn register_member(pubkey: [u8; 32]) {
	assert_ok!(Members::register_member(RawOrigin::Root.into(), ETHEREUM, pubkey.into(), pubkey,));
}

#[test]
fn shard_offline_automatically_creates_new_shard() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	new_test_ext().execute_with(|| {
		register_member([1u8; 32]);
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		register_member([2u8; 32]);
		MemberOnline::<Test>::insert(&b, ());
		Elections::member_online(&b, ETHEREUM);
		register_member([3u8; 32]);
		MemberOnline::<Test>::insert(&c, ());
		Elections::member_online(&c, ETHEREUM);
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(0, ETHEREUM).into());
		Elections::shard_offline(ETHEREUM, [a, b, c].to_vec());
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(1, ETHEREUM).into());
	});
}
