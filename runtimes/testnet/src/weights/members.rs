
//! Autogenerated weights for `pallet_members`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-10-03, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet_members
// --extrinsic
// *
// --output
// ./weights/members.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_members`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_members::WeightInfo for WeightInfo<T> {
	/// Storage: `Members::MemberNetwork` (r:1 w:1)
	/// Proof: `Members::MemberNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Shards::MemberShard` (r:1 w:0)
	/// Proof: `Shards::MemberShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Electable` (r:1 w:0)
	/// Proof: `Elections::Electable` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Unassigned` (r:0 w:1)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::Heartbeat` (r:0 w:1)
	/// Proof: `Members::Heartbeat` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberOnline` (r:0 w:1)
	/// Proof: `Members::MemberOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPublicKey` (r:0 w:1)
	/// Proof: `Members::MemberPublicKey` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberStake` (r:0 w:1)
	/// Proof: `Members::MemberStake` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPeerId` (r:0 w:1)
	/// Proof: `Members::MemberPeerId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn register_member() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `244`
		//  Estimated: `3709`
		// Minimum execution time: 64_930_000 picoseconds.
		Weight::from_parts(69_520_000, 0)
			.saturating_add(Weight::from_parts(0, 3709))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Members::MemberNetwork` (r:1 w:0)
	/// Proof: `Members::MemberNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberOnline` (r:1 w:0)
	/// Proof: `Members::MemberOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::Heartbeat` (r:0 w:1)
	/// Proof: `Members::Heartbeat` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn send_heartbeat() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `293`
		//  Estimated: `3758`
		// Minimum execution time: 22_082_000 picoseconds.
		Weight::from_parts(24_295_000, 0)
			.saturating_add(Weight::from_parts(0, 3758))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Members::MemberNetwork` (r:1 w:1)
	/// Proof: `Members::MemberNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberStake` (r:1 w:1)
	/// Proof: `Members::MemberStake` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Shards::MemberShard` (r:1 w:0)
	/// Proof: `Shards::MemberShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Unassigned` (r:0 w:1)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::Heartbeat` (r:0 w:1)
	/// Proof: `Members::Heartbeat` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberOnline` (r:0 w:1)
	/// Proof: `Members::MemberOnline` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPublicKey` (r:0 w:1)
	/// Proof: `Members::MemberPublicKey` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPeerId` (r:0 w:1)
	/// Proof: `Members::MemberPeerId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn unregister_member() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `380`
		//  Estimated: `3845`
		// Minimum execution time: 58_329_000 picoseconds.
		Weight::from_parts(61_045_000, 0)
			.saturating_add(Weight::from_parts(0, 3845))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(8))
	}
}
