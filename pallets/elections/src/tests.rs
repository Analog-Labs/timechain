use crate::{mock::*, Unassigned};
use time_primitives::{ElectionsInterface, MemberEvents, Network};

#[test]
fn new_member_online_inserts_unassigned() {
	let a: AccountId = [1u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, Network::Ethereum);
		assert!(Unassigned::<Test>::get(Network::Ethereum, &a).is_some());
	});
}

#[test]
fn shard_size_new_members_online_creates_shard() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, Network::Ethereum);
		assert!(Unassigned::<Test>::get(Network::Ethereum, &a).is_some());
		Elections::member_online(&b, Network::Ethereum);
		assert!(Unassigned::<Test>::get(Network::Ethereum, &b).is_some());
		Elections::member_online(&c, Network::Ethereum);
		System::assert_last_event(
			pallet_shards::Event::<Test>::ShardCreated(0, Network::Ethereum).into(),
		);
		for member in [a, b, c] {
			assert!(Unassigned::<Test>::get(Network::Ethereum, member).is_none());
		}
		assert_eq!(pallet_shards::ShardThreshold::<Test>::get(0), Some(2));
	});
}

#[test]
fn member_offline_removes_unassigned() {
	let a: AccountId = [1u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, Network::Ethereum);
		assert!(Unassigned::<Test>::get(Network::Ethereum, &a).is_some());
		Elections::member_offline(&a, Network::Ethereum);
		assert!(Unassigned::<Test>::get(Network::Ethereum, &a).is_none());
	});
}

#[test]
fn shard_offline_automatically_creates_new_shard() {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	new_test_ext().execute_with(|| {
		Elections::member_online(&a, Network::Ethereum);
		Elections::member_online(&b, Network::Ethereum);
		Elections::member_online(&c, Network::Ethereum);
		System::assert_last_event(
			pallet_shards::Event::<Test>::ShardCreated(0, Network::Ethereum).into(),
		);
		Elections::shard_offline(Network::Ethereum, [a, b, c].to_vec());
		System::assert_last_event(
			pallet_shards::Event::<Test>::ShardCreated(1, Network::Ethereum).into(),
		);
	});
}
