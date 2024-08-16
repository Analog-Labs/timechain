use crate::mock::*;
use crate::{Event, ShardMembers, ShardNetwork, ShardState};

use polkadot_sdk::{frame_support, frame_system, pallet_balances, sp_core};

use frame_support::assert_ok;
use frame_support::traits::{Currency, Get};
use frame_system::RawOrigin;

use schnorr_evm::k256::elliptic_curve::PrimeField;
use schnorr_evm::k256::{ProjectivePoint, Scalar};
use schnorr_evm::proof_of_knowledge::construct_proof_of_knowledge;
use schnorr_evm::VerifyingKey;

use time_primitives::{
	AccountId, MemberEvents, NetworkId, PeerId, PublicKey, ShardId, ShardStatus, ShardsInterface,
};

const ETHEREUM: NetworkId = 0;

struct Member {
	account_id: AccountId,
	peer_id: PeerId,
	scalar: Scalar,
	public_key: [u8; 33],
}

fn public_key(acc: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(acc))
}

impl Member {
	pub fn new(i: u8) -> Self {
		let scalar = Scalar::from_repr([i; 32].into()).unwrap();
		Self {
			account_id: [i; 32].into(),
			peer_id: [i; 32],
			scalar,
			public_key: VerifyingKey::new(ProjectivePoint::GENERATOR * scalar).to_bytes().unwrap(),
		}
	}

	fn commitment(&self, threshold: u16) -> Vec<[u8; 33]> {
		vec![self.public_key; threshold as usize]
	}

	fn proof_of_knowledge(&self) -> [u8; 65] {
		construct_proof_of_knowledge(&self.peer_id, &[self.scalar], &[self.public_key]).unwrap()
	}
}

fn shards() -> [[Member; 3]; 2] {
	let a = Member::new(1);
	let b = Member::new(2);
	let c = Member::new(3);
	let d = Member::new(4);
	let e = Member::new(5);
	let f = Member::new(6);
	[[a, b, c], [d, e, f]]
}

fn shard() -> [Member; 3] {
	let a = Member::new(1);
	let b = Member::new(2);
	let c = Member::new(3);
	[a, b, c]
}

fn create_shard(shard_id: ShardId, shard: &[Member], threshold: u16) {
	Shards::create_shard(ETHEREUM, shard.iter().map(|m| m.account_id.clone()).collect(), threshold);
	for member in shard {
		pallet_balances::Pallet::<Test>::resolve_creating(
			&member.account_id,
			pallet_balances::Pallet::<Test>::issue(
				<<Test as pallet_members::Config>::MinStake as Get<u128>>::get() * 100u128,
			),
		);
		assert_ok!(Members::register_member(
			RawOrigin::Signed(member.account_id.clone()).into(),
			ETHEREUM,
			public_key(member.peer_id),
			member.peer_id,
			<Test as pallet_members::Config>::MinStake::get(),
		));
		assert_ok!(Shards::commit(
			RawOrigin::Signed(member.account_id.clone()).into(),
			shard_id,
			member.commitment(threshold),
			member.proof_of_knowledge(),
		));
	}
	for member in shard {
		assert_ok!(Shards::ready(RawOrigin::Signed(member.account_id.clone()).into(), shard_id));
	}
}

#[test]
fn test_register_shard() {
	let shards = shards();
	new_test_ext().execute_with(|| {
		for shard in &shards {
			Shards::create_shard(ETHEREUM, shard.iter().map(|m| m.account_id.clone()).collect(), 1);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			let members = Shards::get_shard_members(shard_id as _);
			let threshold = Shards::get_shard_threshold(shard_id as _);
			assert_eq!(members.len(), shard.len());
			assert_eq!(threshold, 1);
		}
		for member in shard() {
			let shards = Shards::get_shards(&member.account_id);
			assert_eq!(shards.len(), 1);
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			let threshold = Shards::get_shard_threshold(shard_id as _);
			for member in shard {
				pallet_balances::Pallet::<Test>::resolve_creating(
					&member.account_id,
					pallet_balances::Pallet::<Test>::issue(
						<<Test as pallet_members::Config>::MinStake as Get<u128>>::get() * 100u128,
					),
				);
				assert_ok!(Members::register_member(
					RawOrigin::Signed(member.account_id.clone()).into(),
					ETHEREUM,
					public_key(member.peer_id),
					member.peer_id,
					<Test as pallet_members::Config>::MinStake::get(),
				));
				assert_ok!(Shards::commit(
					RawOrigin::Signed(member.account_id.clone()).into(),
					shard_id as _,
					member.commitment(threshold),
					member.proof_of_knowledge(),
				));
			}
		}
		for (shard_id, shard) in shards.iter().enumerate() {
			for member in shard {
				assert_ok!(Shards::ready(
					RawOrigin::Signed(member.account_id.clone()).into(),
					shard_id as _
				));
			}
		}
	});
}

#[test]
fn dkg_times_out() {
	new_test_ext().execute_with(|| {
		Shards::create_shard(ETHEREUM, shard().iter().map(|m| m.account_id.clone()).collect(), 1);
		roll_to(11);
		System::assert_last_event(Event::<Test>::ShardKeyGenTimedOut(0).into());
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
		assert!(ShardNetwork::<Test>::get(0).is_none());
		assert!(ShardMembers::<Test>::iter().collect::<Vec<_>>().is_empty());
	});
}

#[test]
fn member_offline_above_threshold_sets_online_shard_offline() {
	let shard = shard();
	new_test_ext().execute_with(|| {
		create_shard(0, &shard, 3);
		Shards::member_offline(&shard[0].account_id, ETHEREUM);
		assert_eq!(ShardState::<Test>::get(0), Some(ShardStatus::Offline));
	});
}
