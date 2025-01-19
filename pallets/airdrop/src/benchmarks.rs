//! Benchmarks suite for airdrop pallet

use crate::{Call, Claims, Config, Pallet, RawSignature};

use scale_codec::{Decode, Encode};

use polkadot_sdk::*;

use frame_benchmarking::{account, benchmarks};
use frame_support::traits::UnfilteredDispatchable;
use frame_system::RawOrigin;
use sp_keyring::AccountKeyring::Charlie as SrClaimer;
use sp_keyring::Ed25519Keyring::Ferdie as EdClaimer;

use sp_runtime::{traits::ValidateUnsigned, DispatchResult};

use time_primitives::AccountId;

const SEED: u32 = 0;

const MAX_CLAIMS: u32 = 100_000;
const VALUE: u32 = 100_000;
const VESTING: (u32, u32, u32) = (10_000u32, 1_000u32, 10u32);

fn create_claim<T: Config>(ctx: &'static str, src: u32) -> DispatchResult {
	Pallet::<T>::mint(
		RawOrigin::Root.into(),
		account::<AccountId>(ctx, src, SEED),
		VALUE.into(),
		Some((VESTING.0.into(), VESTING.1.into(), VESTING.2.into())),
	)?;
	Ok(())
}

// Pre-signed signatures, as signing requires std environment
const SCHNORR_PROOF: RawSignature = [
	108, 179, 144, 236, 39, 30, 212, 3, 11, 117, 39, 64, 38, 121, 189, 238, 112, 103, 129, 77, 255,
	173, 147, 32, 47, 83, 46, 237, 153, 16, 125, 73, 79, 50, 247, 171, 147, 109, 240, 151, 248, 84,
	7, 214, 191, 241, 190, 126, 104, 121, 158, 226, 30, 28, 15, 11, 236, 148, 14, 133, 16, 180,
	228, 143,
];
const EDWARDS_PROOF: RawSignature = [
	165, 217, 3, 2, 15, 78, 152, 56, 145, 218, 178, 245, 35, 149, 237, 208, 173, 38, 17, 16, 147,
	215, 132, 161, 58, 34, 92, 97, 132, 191, 187, 236, 127, 126, 202, 232, 97, 253, 190, 15, 5, 24,
	99, 94, 83, 138, 74, 121, 151, 157, 60, 149, 42, 0, 201, 173, 138, 48, 142, 224, 196, 88, 126,
	0,
];

benchmarks! {
	// Benchmark `claim_raw` including `validate_unsigned` logic.
	claim_raw {
		for i in 0 .. MAX_CLAIMS {
			create_claim::<T>("source", i)?;
		}

		let source: AccountId = SrClaimer.into();

		Pallet::<T>::mint(
			RawOrigin::Root.into(),
			source.clone(),
			VALUE.into(),
			Some((VESTING.0.into(), VESTING.1.into(), VESTING.2.into())),
		)?;
		assert_eq!(Claims::<T>::get(source.clone()), Some(VALUE.into()));

		let target: T::AccountId = account("target", 0, SEED);
		// This is how the proof was generate, but requires std
		//let proof = SrClaimer.sign(&Pallet::<T>::to_message(&target)[..]).0;

		let txsource = sp_runtime::transaction_validity::TransactionSource::External;
		let call_enc = Call::<T>::claim_raw { source: source.clone(), proof: SCHNORR_PROOF, target }.encode();
	}: {
		let call = <Call<T> as Decode>::decode(&mut &*call_enc)
			.expect("call is encoded above, encoding must be correct");
		super::Pallet::<T>::validate_unsigned(txsource, &call).map_err(|e| -> &'static str { e.into() })?;
		call.dispatch_bypass_filter(RawOrigin::None.into())?;
	}
	verify {
		assert_eq!(Claims::<T>::get(source), None);
	}

	// Benchmark `claim_raw` including `validate_unsigned` logic, but for edwards curve
	#[extra]
	claim_raw_edwards {
		for i in 0 .. MAX_CLAIMS {
			create_claim::<T>("source", i)?;
		}
		let source: AccountId = EdClaimer.into();

		Pallet::<T>::mint(
			RawOrigin::Root.into(),
			source.clone(),
			VALUE.into(),
			Some((VESTING.0.into(), VESTING.1.into(), VESTING.2.into())),
		)?;
		assert_eq!(Claims::<T>::get(source.clone()), Some(VALUE.into()));

		let target: T::AccountId = account("target", 0, SEED);
		// This is how the proof was generate, but requires std
		//let proof = EdClaimer.sign(&Pallet::<T>::to_message(&target)[..]).0;

		let txsource = sp_runtime::transaction_validity::TransactionSource::External;
		let call_enc = Call::<T>::claim_raw { source: source.clone(), proof: EDWARDS_PROOF, target }.encode();
	}: {
		let call = <Call<T> as Decode>::decode(&mut &*call_enc)
			.expect("call is encoded above, encoding must be correct");
		super::Pallet::<T>::validate_unsigned(txsource, &call).map_err(|e| -> &'static str { e.into() })?;
		call.dispatch_bypass_filter(RawOrigin::None.into())?;
	}
	verify {
		assert_eq!(Claims::<T>::get(source), None);
	}

	// Benchmark `mint_claim` when there already exists `MAX_CLAIMS` claims in storage.
	mint {
		for i in 0 .. MAX_CLAIMS {
			create_claim::<T>("owner", i)?;
		}

		let owner: AccountId = account("owner", MAX_CLAIMS, SEED);
	}: _(RawOrigin::Root, owner.clone(), VALUE.into(), Some((VESTING.0.into(), VESTING.1.into(), VESTING.2.into())))
	verify {
		assert_eq!(Claims::<T>::get(owner), Some(VALUE.into()));
	}

	impl_benchmark_test_suite!(
		Pallet,
		crate::tests::new_test_ext(),
		crate::tests::Test,
	);
}
