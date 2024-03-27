use super::*;
use crate::Pallet;
use frame_benchmarking::benchmarks;
use frame_system::RawOrigin;
use schnorr_evm::k256::ProjectivePoint;
use schnorr_evm::VerifyingKey;
use sp_std::vec;
use time_primitives::{AccountId, NetworkId, ShardsInterface};

pub const ALICE: [u8; 32] = [1u8; 32];
pub const ETHEREUM: NetworkId = 0;

pub fn public_key() -> [u8; 33] {
	VerifyingKey::new(ProjectivePoint::GENERATOR).to_bytes().unwrap()
}

benchmarks! {
	commit {
		let shard: Vec<AccountId> = vec![[0u8; 32].into(), ALICE.into(), [2u8; 32].into()];
		Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1);
		let public_key = public_key();
		let alice: AccountId = ALICE.into();
		// benchmark commitment that changes shard status
		for member in shard {
			if member != alice {
				Pallet::<T>::commit(
					RawOrigin::Signed(member.clone()).into(),
					0,
					vec![public_key.clone()],
					[0; 65],
				)?;
			}
		}
	}: _(RawOrigin::Signed(ALICE.into()), 0, vec![public_key],
	[0; 65])
	verify { }

	ready {
		let shard: Vec<AccountId> = vec![[0u8; 32].into(), ALICE.into(), [2u8; 32].into()];
		Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1);
		let public_key = public_key();
		let alice: AccountId = ALICE.into();
		for member in shard.clone() {
			Pallet::<T>::commit(
				RawOrigin::Signed(member.clone()).into(),
				0,
				vec![public_key.clone()],
				[0; 65],
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
		let shard: Vec<AccountId> = vec![[0u8; 32].into(), ALICE.into(), [2u8; 32].into()];
		Pallet::<T>::create_shard(ETHEREUM, shard.clone(), 1);
	}: _(RawOrigin::Root, 0)
	verify { }

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
