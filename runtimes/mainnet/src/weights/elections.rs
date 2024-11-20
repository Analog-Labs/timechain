
//! Autogenerated weights for `pallet_elections`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-11-20, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
	/// Storage: `Networks::NetworkName` (r:1 w:0)
	/// Proof: `Networks::NetworkName` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardSize` (r:0 w:1)
	/// Proof: `Elections::ShardSize` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardThreshold` (r:0 w:1)
	/// Proof: `Elections::ShardThreshold` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_shard_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `117`
		//  Estimated: `3582`
		// Minimum execution time: 14_125_000 picoseconds.
		Weight::from_parts(19_597_000, 0)
			.saturating_add(Weight::from_parts(0, 3582))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Elections::Unassigned` (r:301 w:300)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberOnline` (r:300 w:0)
	/// Proof: `Members::MemberOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardSize` (r:1 w:0)
	/// Proof: `Elections::ShardSize` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberStake` (r:300 w:0)
	/// Proof: `Members::MemberStake` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardThreshold` (r:1 w:0)
	/// Proof: `Elections::ShardThreshold` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
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
		//  Measured:  `214 + b * (534 ±0)`
		//  Estimated: `3657 + b * (7959 ±0)`
		// Minimum execution time: 79_489_000 picoseconds.
		Weight::from_parts(82_675_000, 0)
			.saturating_add(Weight::from_parts(0, 3657))
			// Standard Error: 747_582
			.saturating_add(Weight::from_parts(214_002_489, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((9_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((14_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 7959).saturating_mul(b.into()))
	}
}
