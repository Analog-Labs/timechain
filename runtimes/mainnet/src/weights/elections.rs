
//! Autogenerated weights for `pallet_elections`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-11-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
		// Minimum execution time: 12_413_000 picoseconds.
		Weight::from_parts(15_249_000, 0)
			.saturating_add(Weight::from_parts(0, 3588))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Elections::Unassigned` (r:257 w:3)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberOnline` (r:256 w:0)
	/// Proof: `Members::MemberOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardSize` (r:1 w:0)
	/// Proof: `Elections::ShardSize` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberStake` (r:256 w:0)
	/// Proof: `Members::MemberStake` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardThreshold` (r:1 w:0)
	/// Proof: `Elections::ShardThreshold` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardIdCounter` (r:1 w:1)
	/// Proof: `Shards::ShardIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::DkgTimeout` (r:0 w:1)
	/// Proof: `Shards::DkgTimeout` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:0 w:1)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:0 w:3)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::MemberShard` (r:0 w:3)
	/// Proof: `Shards::MemberShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembersOnline` (r:0 w:1)
	/// Proof: `Shards::ShardMembersOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardThreshold` (r:0 w:1)
	/// Proof: `Shards::ShardThreshold` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:0 w:1)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[3, 256]`.
	/// The range of component `b` is `[3, 256]`.
	fn try_elect_shard(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `184 + b * (178 ±0)`
		//  Estimated: `3651 + b * (2653 ±0)`
		// Minimum execution time: 75_621_000 picoseconds.
		Weight::from_parts(77_876_000, 0)
			.saturating_add(Weight::from_parts(0, 3651))
			// Standard Error: 95_999
			.saturating_add(Weight::from_parts(43_508_123, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(15))
			.saturating_add(Weight::from_parts(0, 2653).saturating_mul(b.into()))
	}
}
