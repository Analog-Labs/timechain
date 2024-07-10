use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use sp_std::vec;
use sp_std::vec::Vec;
use time_primitives::{AccountId, MemberEvents, NetworkId, ShardsInterface};
pub const ALICE: [u8; 32] = [1u8; 32];
pub const BOB: [u8; 32] = [2u8; 32];
pub const CHARLIE: [u8; 32] = [3u8; 32];
pub const ETHEREUM: NetworkId = 0;
use time_primitives::{Commitment, ProofOfKnowledge};

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
		ALICE => [
			3, 27, 132, 197, 86, 123, 18, 100, 64, 153, 93, 62, 213, 170, 186, 5, 101, 215, 30, 24,
			52, 96, 72, 25, 255, 156, 23, 245, 233, 213, 221, 7, 143,
		],
		BOB => [
			2, 77, 75, 108, 209, 54, 16, 50, 202, 155, 210, 174, 185, 217, 0, 170, 77, 69, 217,
			234, 216, 10, 201, 66, 51, 116, 196, 81, 167, 37, 77, 7, 102,
		],
		CHARLIE => [
			2, 83, 31, 230, 6, 129, 52, 80, 61, 39, 35, 19, 50, 39, 200, 103, 172, 143, 166, 200,
			60, 83, 126, 154, 68, 195, 197, 189, 189, 203, 31, 227, 55,
		],
		_ => {
			panic!("Invalid member")
		},
	};
	vec![commitment]
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
	commit {
		let shard: Vec<AccountId> = vec![ALICE.into(), BOB.into(), CHARLIE.into()];
		Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1);
		let alice: AccountId = ALICE.into();
		// benchmark commitment that changes shard status
		for member in shard {
			if member != alice {
				Pallet::<T>::commit(
					RawOrigin::Signed(member.clone()).into(),
					0,
					get_commitment(member.clone().into()),
					get_proof_of_knowledge(member.into()),
				)?;
			}
		}
	}: _(RawOrigin::Signed(ALICE.into()), 0, get_commitment(ALICE),
	get_proof_of_knowledge(ALICE))
	verify { }

	ready {
		let shard: Vec<AccountId> = vec![ALICE.into(), BOB.into(), CHARLIE.into()];
		Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1);
		let alice: AccountId = ALICE.into();
		for member in shard.clone() {
			Pallet::<T>::commit(
				RawOrigin::Signed(member.clone()).into(),
				0,
				get_commitment(member.clone().into()),
				get_proof_of_knowledge(member.into()),
			)?;
		}
		// benchmark ready that changes shard status
		for member in shard {
			if member != alice {
				Pallet::<T>::ready(
					RawOrigin::Signed(member.clone()).into(),
					0,
				)?;
			}
		}
	}: _(RawOrigin::Signed(alice), 0)
	verify { }

	force_shard_offline {
		let shard: Vec<AccountId> = vec![ALICE.into(), BOB.into(), CHARLIE.into()];
		Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1);
	}: _(RawOrigin::Root, 0)
	verify { }

	member_offline {
		let member: AccountId = ALICE.into();
		Pallet::<T>::member_online(&member, ETHEREUM);
	}: {
		Pallet::<T>::member_offline(&member, ETHEREUM);
	} verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
