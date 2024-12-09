use super::*;
use crate::{Config, Pallet};

use frame_benchmarking::benchmarks;
use frame_support::{
	assert_ok,
	traits::{Currency, Get},
};
use frame_system::RawOrigin;
use polkadot_sdk::{
	frame_benchmarking, frame_support, frame_system, pallet_balances, sp_core, sp_runtime, sp_std,
};
use sp_runtime::{BoundedVec, Saturating};

use sp_std::vec;
use sp_std::vec::Vec;

use time_primitives::{
	AccountId, Commitment, ElectionsInterface, NetworkId, ProofOfKnowledge, PublicKey, ShardStatus,
	ShardsInterface,
};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const CHARLIE: [u8; 32] = [3u8; 32];
pub const ETHEREUM: NetworkId = 0;
const SHARD_ID: u64 = 0;

fn public_key(acc: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(acc))
}

/// Since benchmarks are no-std and we need std computation on constructing proof so
/// these values are taken by running the code in pallets/shards/src/tests.rs
/// in a seperate tests and then taking the data from commitment and pok and using them here.

pub const ALICE_COMMITMENT: [u8; 33] = [
	3, 27, 132, 197, 86, 123, 18, 100, 64, 153, 93, 62, 213, 170, 186, 5, 101, 215, 30, 24, 52, 96,
	72, 25, 255, 156, 23, 245, 233, 213, 221, 7, 143,
];
pub const BOB_COMMITMENT: [u8; 33] = [
	2, 77, 75, 108, 209, 54, 16, 50, 202, 155, 210, 174, 185, 217, 0, 170, 77, 69, 217, 234, 216,
	10, 201, 66, 51, 116, 196, 81, 167, 37, 77, 7, 102,
];
pub const CHARLIE_COMMITMENT: [u8; 33] = [
	2, 83, 31, 230, 6, 129, 52, 80, 61, 39, 35, 19, 50, 39, 200, 103, 172, 143, 166, 200, 60, 83,
	126, 154, 68, 195, 197, 189, 189, 203, 31, 227, 55,
];
pub const ALICE_POK: ProofOfKnowledge = [
	2, 130, 142, 156, 2, 13, 251, 97, 156, 208, 17, 0, 246, 123, 48, 97, 171, 221, 27, 179, 198,
	196, 203, 31, 198, 249, 181, 23, 239, 34, 197, 193, 206, 128, 187, 250, 214, 213, 204, 223,
	189, 100, 9, 39, 120, 37, 224, 56, 143, 248, 172, 139, 127, 95, 54, 51, 144, 241, 239, 134, 86,
	214, 45, 191, 237,
];
pub const BOB_POK: ProofOfKnowledge = [
	2, 228, 146, 211, 147, 253, 131, 230, 153, 86, 219, 237, 174, 73, 215, 139, 20, 38, 17, 79,
	100, 229, 78, 191, 233, 254, 98, 92, 213, 100, 162, 70, 79, 205, 137, 138, 183, 209, 11, 130,
	174, 14, 171, 144, 83, 20, 227, 233, 15, 27, 247, 11, 145, 142, 24, 41, 88, 80, 78, 175, 163,
	130, 204, 191, 137,
];
pub const CHARLIE_POK: ProofOfKnowledge = [
	3, 57, 183, 166, 234, 228, 13, 139, 136, 54, 192, 207, 166, 77, 49, 160, 100, 97, 84, 244, 204,
	111, 158, 214, 72, 185, 254, 195, 139, 87, 231, 148, 76, 186, 107, 203, 85, 70, 175, 100, 117,
	123, 134, 165, 76, 77, 20, 6, 24, 216, 29, 182, 93, 75, 80, 152, 224, 73, 163, 34, 141, 42, 58,
	147, 134,
];

pub fn get_commitment(member: [u8; 32]) -> Commitment {
	let commitment = match member {
		ALICE => ALICE_COMMITMENT,
		BOB => BOB_COMMITMENT,
		CHARLIE => CHARLIE_COMMITMENT,
		_ => {
			panic!("Invalid member")
		},
	};
	Commitment(BoundedVec::truncate_from(vec![commitment]))
}
pub fn get_proof_of_knowledge(member: [u8; 32]) -> ProofOfKnowledge {
	match member {
		ALICE => ALICE_POK,
		BOB => BOB_POK,
		CHARLIE => CHARLIE_POK,
		_ => {
			panic!("Invalid member")
		},
	}
}

benchmarks! {
	where_clause {  where T: pallet_members::Config }
	commit {
		let shard: Vec<[u8; 32]> = vec![ALICE, BOB, CHARLIE];
		assert_ok!(Pallet::<T>::create_shard(ETHEREUM, shard.clone().into_iter().map(|x| x.into()).collect::<Vec<AccountId>>(), 1));
		let alice: AccountId = ALICE.into();
		// benchmark commitment that changes shard status
		for member in shard {
			let member_account: AccountId = member.into();
			pallet_balances::Pallet::<T>::resolve_creating(
				&member_account,
				pallet_balances::Pallet::<T>::issue(<T as pallet_members::Config>::MinStake::get() * 100),
			);
			pallet_members::Pallet::<T>::register_member(
				RawOrigin::Signed(member_account.clone()).into(),
				ETHEREUM,
				public_key(member),
				member,
				<T as pallet_members::Config>::MinStake::get(),
			)?;
			if member != ALICE {
				Pallet::<T>::commit(
					RawOrigin::Signed(member_account.clone()).into(),
					0,
					get_commitment(member_account.clone().into()),
					get_proof_of_knowledge(member_account.into()),
				)?;
			}
		}
	}: _(RawOrigin::Signed(ALICE.into()), SHARD_ID, get_commitment(ALICE),
	get_proof_of_knowledge(ALICE))
	verify { }

	ready {
		let shard: Vec<[u8; 32]> = vec![ALICE, BOB, CHARLIE];
		assert_ok!(Pallet::<T>::create_shard(ETHEREUM, shard.clone().into_iter().map(|x| x.into()).collect::<Vec<AccountId>>(), 1));
		for member in shard.clone() {
			let member_account: AccountId = member.into();
			pallet_balances::Pallet::<T>::resolve_creating(
				&member_account,
				pallet_balances::Pallet::<T>::issue(<T as pallet_members::Config>::MinStake::get() * 100),
			);
			pallet_members::Pallet::<T>::register_member(
				RawOrigin::Signed(member_account.clone()).into(),
				ETHEREUM,
				public_key(member),
				member,
				<T as pallet_members::Config>::MinStake::get(),
			)?;
			Pallet::<T>::commit(
				RawOrigin::Signed(member_account.clone()).into(),
				0,
				get_commitment(member_account.clone().into()),
				get_proof_of_knowledge(member_account.into()),
			)?;
		}
		// benchmark ready that changes shard status
		for member in shard {
			if member != ALICE {
				Pallet::<T>::ready(
					RawOrigin::Signed(member.into()).into(),
					SHARD_ID,
				)?;
			}
		}
	}: _(RawOrigin::Signed(ALICE.into()), SHARD_ID)
	verify { }

	force_shard_offline {
		let shard: Vec<AccountId> = vec![ALICE.into(), BOB.into(), CHARLIE.into()];
		assert_ok!(Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1));
	}: _(RawOrigin::Root, SHARD_ID)
	verify { }

	timeout_dkgs {
		let b in 1..<<T as Config>::Elections as ElectionsInterface>::MaxElectionsPerBlock::get();
		for i in 0..b {
			assert_ok!(Pallet::<T>::create_shard(ETHEREUM, vec![ALICE.into(), BOB.into(), CHARLIE.into()], 1));
			assert_eq!(ShardState::<T>::get(i as u64), Some(ShardStatus::Created));
		}
		let timeout_block = frame_system::Pallet::<T>::block_number().saturating_add(T::DkgTimeout::get());
	}: {
		Pallet::<T>::timeout_dkgs(timeout_block);
	} verify {
		for i in 0..b {
			assert_eq!(ShardState::<T>::get(i as u64), Some(ShardStatus::Offline));
		}
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
