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
		Elections::member_offline(&a, ETHEREUM);
		assert!(!Unassigned::<Test>::get(ETHEREUM).contains(&a));
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
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		register_member([2u8; 32], 5);
		MemberOnline::<Test>::insert(&b, ());
		Elections::member_online(&b, ETHEREUM);
		register_member([3u8; 32], 5);
		MemberOnline::<Test>::insert(&c, ());
		Elections::member_online(&c, ETHEREUM);
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(0, ETHEREUM).into());
		Elections::shard_offline(ETHEREUM, [a, b, c].to_vec());
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(1, ETHEREUM).into());
	});
}

#[test]
fn member_selection_in_order_of_member_stake() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	let d: AccountId = [4u8; 32].into();
	new_test_ext().execute_with(|| {
		register_member([1u8; 32], 8);
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		register_member([2u8; 32], 7);
		MemberOnline::<Test>::insert(&b, ());
		Elections::member_online(&b, ETHEREUM);
		register_member([3u8; 32], 6);
		MemberOnline::<Test>::insert(&c, ());
		Elections::member_online(&c, ETHEREUM);
		// expect lowest balance = 5 to be unassigned
		register_member([4u8; 32], 5);
		MemberOnline::<Test>::insert(&d, ());
		Elections::member_online(&d, ETHEREUM);
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(0, ETHEREUM).into());
		for member in [a, b, c] {
			// highest 3/4 balances are assigned to shard election
			assert!(!Unassigned::<Test>::get(ETHEREUM).contains(&member));
		}
		// lowest balance = 5 is unassigned after shard election
		assert!(Unassigned::<Test>::get(ETHEREUM).contains(&d));
	});
}

#[test]
fn shard_selection_in_order_of_member_stake() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	let d: AccountId = [4u8; 32].into();
	let e: AccountId = [5u8; 32].into();
	let f: AccountId = [6u8; 32].into();
	let g: AccountId = [7u8; 32].into();
	let h: AccountId = [8u8; 32].into();
	new_test_ext().execute_with(|| {
		register_member([1u8; 32], 11);
		MemberOnline::<Test>::insert(&a, ());
		Elections::member_online(&a, ETHEREUM);
		register_member([2u8; 32], 10);
		MemberOnline::<Test>::insert(&b, ());
		Elections::member_online(&b, ETHEREUM);
		register_member([3u8; 32], 9);
		MemberOnline::<Test>::insert(&c, ());
		Elections::member_online(&c, ETHEREUM);
		register_member([4u8; 32], 8);
		MemberOnline::<Test>::insert(&d, ());
		Elections::member_online(&d, ETHEREUM);
		register_member([5u8; 32], 7);
		MemberOnline::<Test>::insert(&e, ());
		Elections::member_online(&e, ETHEREUM);
		register_member([6u8; 32], 6);
		MemberOnline::<Test>::insert(&f, ());
		Elections::member_online(&f, ETHEREUM);
		register_member([7u8; 32], 5);
		MemberOnline::<Test>::insert(&g, ());
		Elections::member_online(&g, ETHEREUM);
		register_member([8u8; 32], 5);
		MemberOnline::<Test>::insert(&h, ());
		Elections::member_online(&h, ETHEREUM);
		roll(1);
		System::assert_last_event(pallet_shards::Event::<Test>::ShardCreated(1, ETHEREUM).into());
		for member in [a, b, c, d, e, f] {
			// highest 6/8 balances are assigned to shard election
			assert!(!Unassigned::<Test>::get(ETHEREUM).contains(&member));
		}
		for member in [g, h] {
			// lowest 2/8 balances are unassigned
			assert!(Unassigned::<Test>::get(ETHEREUM).contains(&member));
		}
	});
}
