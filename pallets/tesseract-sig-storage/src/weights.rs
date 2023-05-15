
//! Autogenerated weights for `pallet_tesseract_sig_storage`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-04-06, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-50-203`, CPU: `Intel(R) Xeon(R) Platinum 8259CL CPU @ 2.50GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/timechain-node
// benchmark
// pallet
// --chain
// dev
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet
// pallet-tesseract-sig-storage
// --extrinsic
// 
// --steps
// 50
// --repeat
// 20
// --output
// pallets/tesseract-sig-storage/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_tesseract_sig_storage`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
	/// Storage: TesseractSigStorage SignatureStoreData (r:0 w:1)
	/// Proof Skipped: TesseractSigStorage SignatureStoreData (max_values: None, max_size: None, mode: Measured)
	/// The range of component `s` is `[0, 255]`.
	fn store_signature_data(_s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 58_564_000 picoseconds.
		Weight::from_parts(77_722_783, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: TesseractSigStorage TssGroupKey (r:0 w:1)
	/// Proof Skipped: TesseractSigStorage TssGroupKey (max_values: None, max_size: None, mode: Measured)
	/// The range of component `s` is `[1, 255]`.
	fn submit_tss_group_key(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 59_164_000 picoseconds.
		Weight::from_parts(73_348_700, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 1_109
			.saturating_add(Weight::from_parts(5_709, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().writes(1))
	}

	fn register_shard(_s: u32) -> Weight {
		Weight::from_parts(1_000_000, 0)
	}
}
