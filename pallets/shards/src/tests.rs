use crate::mock::*;
use crate::{Event, ShardMembers, ShardNetwork, ShardState};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use schnorr_evm::k256::ProjectivePoint;
use schnorr_evm::VerifyingKey;
use time_primitives::{
	AccountId, MemberEvents, Network, ShardId, ShardStatus, ShardsInterface, TssPublicKey,
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

fn public_key() -> [u8; 33] {
	VerifyingKey::new(ProjectivePoint::GENERATOR).to_bytes().unwrap()
}

fn create_shard(shard_id: ShardId, shard: &[AccountId], threshold: u16) {
	let public_key = public_key();
	Shards::create_shard(Network::Ethereum, shard.to_vec(), threshold);
	for account in shard {
		assert_ok!(Shards::commit(
			RawOrigin::Signed(account.clone()).into(),
			shard_id,
			vec![TssPublicKey(public_key); threshold as usize],
			[0; 65]
		));
	}
	for account in shard {
		assert_ok!(Shards::ready(RawOrigin::Signed(account.clone()).into(), shard_id));
	}
}

#[test]
fn test_register_shard() {
	let shards = shards();
	let public_key = public_key();
	new_test_ext().execute_with(|| {
		for shard in &shards {
			Shards::create_shard(Network::Ethereum, shard.to_vec(), 1);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			let members = Shards::get_shard_members(shard_id as _);
			let threshold = Shards::get_shard_threshold(shard_id as _);
			assert_eq!(members.len(), shard.len());
			assert_eq!(threshold, 1);
		}
		for member in shard() {
			let shards = Shards::get_shards(&member);
			assert_eq!(shards.len(), 1);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			for account in shard {
				assert_ok!(Shards::commit(
					RawOrigin::Signed(account.clone()).into(),
					shard_id as _,
					vec![TssPublicKey(public_key)],
					[0; 65]
				));
			}
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			for account in shard {
				assert_ok!(Shards::ready(RawOrigin::Signed(account.clone()).into(), shard_id as _));
			}
		}
	});
}

#[test]
fn dkg_times_out() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(Network::Ethereum, shard().to_vec(), 1);
		roll_to(11);
		System::assert_last_event(Event::<Test>::ShardKeyGenTimedOut(0).into());
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
		assert!(ShardNetwork::<Test>::get(0).is_none());
		assert!(ShardMembers::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn member_offline_sets_online_shard_partially_offline() {
	let shard = shard();
	new_test_ext().execute_with(|| {
		create_shard(0, &shard, 1);
		Shards::member_offline(&shard[0], Network::Ethereum);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::PartialOffline(1)));
	});
}

#[test]
fn member_offline_above_threshold_sets_online_shard_offline() {
	let shard = shard();
	new_test_ext().execute_with(|| {
		create_shard(0, &shard, 3);
		Shards::member_offline(&shard[0], Network::Ethereum);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
	});
}

#[test]
fn member_online_sets_partially_offline_shard_back_online() {
	let shard = shard();
	new_test_ext().execute_with(|| {
		create_shard(0, &shard, 1);
		Shards::member_offline(&shard[0], Network::Ethereum);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::PartialOffline(1)));
		Shards::member_online(&shard[0], Network::Ethereum);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Online));
	});
}
