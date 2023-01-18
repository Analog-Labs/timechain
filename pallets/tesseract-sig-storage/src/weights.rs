
//! Autogenerated weights for `pallet_tesseract_sig_storage`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-10-05, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/timechain-node
// benchmark
// pallet
// --chain
// dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet
// pallet_tesseract_sig_storage
// --extrinsic
// store_signature
// --steps
// 50
// --repeat
// 20
// --output
// pallets/tesseract-sig-storage/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]


//use frame_support::{traits::Get, weights::Weight};
use frame_support::{traits::Get, weights::{constants::RocksDbWeight,Weight}};
use crate::WeightInfo;
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_tesseract_sig_storage`.

pub struct SigWeightInfo<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SigWeightInfo<T> {
	// Storage: TesseractSigStorage TesseractMembers (r:0 w:1)
    fn add_member() -> Weight {
        Weight::from_ref_time(33_000_000_u64)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    // Storage: TesseractSigStorage TesseractMembers (r:1 w:0)
    // Storage: TesseractSigStorage SignatureStore (r:0 w:1)
    /// The range of component `s` is `[0, 100]`.

    fn store_signature_data() -> Weight {
        Weight::from_ref_time(39_000_000_u64)
            // Standard Error: 2_151
            .saturating_add(Weight::from_ref_time(68_175_u64).saturating_mul(100_u64))
            .saturating_add(T::DbWeight::get().reads(1_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    // Storage: TesseractSigStorage TesseractMembers (r:0 w:1)
    fn remove_member() -> Weight {
        Weight::from_ref_time(33_000_000_u64)
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    // Storage: TesseractSigStorage TssGroupKey (r:0 w:1)
	fn submit_tss_group_key() -> Weight {
		Weight::from_ref_time(3_469_000_000_u64)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}

}


impl WeightInfo for () {

    // Storage: TesseractSigStorage TesseractMembers (r:0 w:1)
    fn add_member() -> Weight {
        Weight::from_ref_time(33_000_000_u64)
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }

    fn store_signature_data() -> Weight {
		Weight::from_ref_time(33_000_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}

	   // Storage: TesseractSigStorage TesseractMembers (r:0 w:1)
	   fn remove_member() -> Weight {
        Weight::from_ref_time(33_000_000_u64)
            .saturating_add(RocksDbWeight::get().writes(1_u64))
    }
    
    // Storage: TesseractSigStorage TssGroupKey (r:0 w:1)
	fn submit_tss_group_key() -> Weight {
		Weight::from_ref_time(3_469_000_000_u64)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
    
}