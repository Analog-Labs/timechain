
//! Autogenerated weights for `pallet_elections`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-07-24, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `benchmark-agent-1`, CPU: `AMD EPYC Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("development")`, DB CACHE: 1024

// Executed Command:
// ./timechain-node
// benchmark
// pallet
// --chain
// development
// --pallet
// pallet_elections
// --extrinsic
// *
// --output
// ./weights/elections.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_elections`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_elections::WeightInfo for WeightInfo<T> {
	/// Storage: `Elections::Unassigned` (r:1 w:0)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardSize` (r:0 w:1)
	/// Proof: `Elections::ShardSize` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardThreshold` (r:0 w:1)
	/// Proof: `Elections::ShardThreshold` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_shard_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `123`
		//  Estimated: `3588`
		// Minimum execution time: 12_503_000 picoseconds.
		Weight::from_parts(14_808_000, 0)
			.saturating_add(Weight::from_parts(0, 3588))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}
