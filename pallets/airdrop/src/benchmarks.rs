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
	228, 228, 235, 104, 177, 228, 253, 56, 228, 202, 232, 246, 73, 225, 46, 255, 42, 183, 192, 71,
	17, 247, 8, 124, 172, 178, 146, 79, 159, 76, 118, 74, 250, 154, 50, 20, 73, 205, 134, 93, 20,
	44, 161, 80, 111, 116, 202, 143, 214, 175, 159, 133, 173, 35, 142, 105, 25, 5, 163, 162, 46,
	193, 12, 137,
];
const EDWARDS_PROOF: RawSignature = [
	0, 40, 229, 252, 215, 171, 119, 94, 235, 66, 105, 70, 231, 4, 128, 211, 217, 232, 47, 194, 27,
	204, 221, 202, 63, 127, 133, 50, 2, 245, 106, 47, 213, 198, 114, 141, 216, 57, 126, 77, 20,
	163, 234, 204, 5, 196, 252, 122, 24, 209, 155, 87, 185, 74, 73, 35, 197, 216, 245, 201, 247,
	142, 66, 7,
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
