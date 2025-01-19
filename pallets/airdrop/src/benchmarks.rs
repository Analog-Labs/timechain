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
	46, 61, 191, 97, 42, 149, 19, 91, 169, 167, 82, 114, 64, 62, 141, 4, 205, 129, 88, 79, 2, 219,
	168, 171, 212, 148, 212, 50, 174, 145, 213, 57, 8, 139, 43, 137, 72, 172, 70, 246, 145, 5, 4,
	22, 212, 107, 72, 72, 208, 32, 25, 82, 80, 247, 132, 1, 181, 208, 86, 220, 248, 166, 185, 130,
];
const EDWARDS_PROOF: RawSignature = [
	65, 15, 58, 88, 56, 207, 111, 92, 69, 45, 52, 127, 5, 32, 116, 253, 35, 25, 22, 169, 228, 142,
	91, 131, 226, 131, 79, 231, 77, 251, 158, 88, 241, 34, 136, 20, 153, 197, 26, 237, 8, 244, 63,
	14, 72, 93, 129, 186, 106, 38, 137, 245, 156, 163, 221, 195, 146, 205, 33, 77, 232, 183, 86, 5,
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
