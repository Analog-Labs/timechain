
//! Autogenerated weights for `pallet_dmail`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-11-04, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `benchmark-agent-1`, CPU: `AMD EPYC Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// ./timechain-node
// benchmark
// pallet
// --chain
// dev
// --pallet
// pallet_dmail
// --extrinsic
// *
// --output
// ./weights/dmail.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_dmail`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_dmail::WeightInfo for WeightInfo<T> {
	fn send_email() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_545_000 picoseconds.
		Weight::from_parts(11_812_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
}
