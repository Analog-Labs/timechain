
//! Autogenerated weights for `pallet_elections`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-12-04, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ns1026992`, CPU: `AMD EPYC 4244P 6-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// ./target/production/timechain-node
// benchmark
// pallet
// --pallet
// pallet_elections
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --output
// ./production_weights/elections.rs

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
	/// Storage: `Elections::ShardSize` (r:1 w:0)
	/// Proof: `Elections::ShardSize` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardThreshold` (r:1 w:0)
	/// Proof: `Elections::ShardThreshold` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Unassigned` (r:1 w:1)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardIdCounter` (r:1 w:1)
	/// Proof: `Shards::ShardIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::DkgTimeout` (r:0 w:100)
	/// Proof: `Shards::DkgTimeout` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:0 w:100)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:0 w:300)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::MemberShard` (r:0 w:300)
	/// Proof: `Shards::MemberShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembersOnline` (r:0 w:100)
	/// Proof: `Shards::ShardMembersOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardThreshold` (r:0 w:100)
	/// Proof: `Shards::ShardThreshold` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:0 w:100)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 100]`.
	/// The range of component `b` is `[1, 100]`.
	fn try_elect_shards(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `173 + b * (96 ±0)`
		//  Estimated: `3637 + b * (96 ±0)`
		// Minimum execution time: 18_194_000 picoseconds.
		Weight::from_parts(9_847_440, 0)
			.saturating_add(Weight::from_parts(0, 3637))
			// Standard Error: 10_051
			.saturating_add(Weight::from_parts(12_527_933, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((11_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 96).saturating_mul(b.into()))
	}
}
