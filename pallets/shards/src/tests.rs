use crate::mock::*;
use crate::{Error, Event, ShardMembers, ShardNetwork, ShardState};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::{
	AccountId, MemberEvents, Network, OcwShardInterface, PublicKey, ShardStatus,
};

fn shards() -> [[AccountId; 3]; 2] {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	let d: AccountId = [4u8; 32].into();
	let e: AccountId = [5u8; 32].into();
	let f: AccountId = [6u8; 32].into();
	[[a, b, c], [d, e, f]]
}

fn shard() -> [AccountId; 3] {
	let a: AccountId = [1u8; 32].into();
	let b: AccountId = [2u8; 32].into();
	let c: AccountId = [3u8; 32].into();
	[a, b, c]
}

fn collector() -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw([42; 32]))
}

#[test]
fn test_register_shard() {
	let shards = shards();
	new_test_ext().execute_with(|| {
		for shard in &shards {
			assert_ok!(Shards::register_shard(
				RawOrigin::Root.into(),
				Network::Ethereum,
				shard.to_vec(),
				collector(),
				1,
			),);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			let members = Shards::get_shard_members(shard_id as _);
			let threshold = Shards::get_shard_threshold(shard_id as _);
			assert_eq!(members.len(), shard.len());
			assert_eq!(threshold, 1);
		}
		for member in shard() {
			let shards = Shards::get_shards(member);
			assert_eq!(shards.len(), 1);
		}
		for (shard_id, _) in shards.iter().enumerate() {
			assert_ok!(Shards::submit_tss_public_key(shard_id as _, [0; 33]));
		}
	});
}

#[test]
fn member_restricted_to_max_one_shard() {
	let shards = shards();
	new_test_ext().execute_with(|| {
		for shard in &shards {
			assert_ok!(Shards::register_shard(
				RawOrigin::Root.into(),
				Network::Ethereum,
				shard.to_vec(),
				collector(),
				1,
			),);
			assert_noop!(
				Shards::register_shard(
					RawOrigin::Root.into(),
					Network::Ethereum,
					shard.to_vec(),
					collector(),
					1,
				),
				Error::<Test>::MaxOneShardPerMember
			);
		}
	});
}

#[test]
fn cannot_submit_public_key_if_shard_not_exists() {
	let shards = shards();
	new_test_ext().execute_with(|| {
		for (shard_id, _) in shards.iter().enumerate() {
			assert_noop!(
				Shards::submit_tss_public_key(shard_id as _, [0; 33]),
				Error::<Test>::UnknownShard
			);
		}
	});
}

#[test]
fn submit_public_key_max_once() {
	let shards = shards();
	new_test_ext().execute_with(|| {
		for shard in &shards {
			assert_ok!(Shards::register_shard(
				RawOrigin::Root.into(),
				Network::Ethereum,
				shard.to_vec(),
				collector(),
				1,
			),);
		}
		for (shard_id, _) in shards.iter().enumerate() {
			assert_ok!(Shards::submit_tss_public_key(shard_id as _, [0; 33]));
			assert_noop!(
				Shards::submit_tss_public_key(shard_id as _, [1; 33]),
				Error::<Test>::PublicKeyAlreadyRegistered
			);
		}
	});
}

#[test]
fn dkg_times_out() {
	new_test_ext().execute_with(|| {
		assert_ok!(Shards::register_shard(
			RawOrigin::Root.into(),
			Network::Ethereum,
			shard().to_vec(),
			collector(),
			1,
		));
		roll_to(11);
		System::assert_last_event(Event::<Test>::ShardKeyGenTimedOut(0).into());
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
		assert!(ShardNetwork::<Test>::get(0).is_none());
		assert!(ShardMembers::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn member_offline_before_group_key_submitted_then_shard_offline() {
	new_test_ext().execute_with(|| {
		assert_ok!(Shards::register_shard(
			RawOrigin::Root.into(),
			Network::Ethereum,
			shard().to_vec(),
			collector(),
			1,
		));
		Shards::member_offline(&shard()[0]);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
		assert_noop!(
			Shards::submit_tss_public_key(0, [0; 33]),
			Error::<Test>::OfflineShardMayNotGoOnline
		);
	});
}

#[test]
fn member_offline_sets_online_shard_partially_offline() {
	new_test_ext().execute_with(|| {
		assert_ok!(Shards::register_shard(
			RawOrigin::Root.into(),
			Network::Ethereum,
			shard().to_vec(),
			collector(),
			1,
		));
		assert_ok!(Shards::submit_tss_public_key(0, [0; 33]));
		Shards::member_offline(&shard()[0]);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::PartialOffline(1)));
	});
}

#[test]
fn member_offline_above_threshold_sets_online_shard_offline() {
	new_test_ext().execute_with(|| {
		assert_ok!(Shards::register_shard(
			RawOrigin::Root.into(),
			Network::Ethereum,
			shard().to_vec(),
			collector(),
			shard().len().try_into().unwrap(),
		));
		assert_ok!(Shards::submit_tss_public_key(0, [0; 33]));
		Shards::member_offline(&shard()[0]);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
	});
}
