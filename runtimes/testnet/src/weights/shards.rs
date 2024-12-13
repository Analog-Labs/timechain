
//! Autogenerated weights for `pallet_shards`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-12-13, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet_shards
// --extrinsic
// *
// --output
// ./weights/shards.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_shards`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_shards::WeightInfo for WeightInfo<T> {
	/// Storage: `Shards::ShardMembers` (r:4 w:1)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardThreshold` (r:1 w:0)
	/// Proof: `Shards::ShardThreshold` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPeerId` (r:1 w:0)
	/// Proof: `Members::MemberPeerId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:0 w:1)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:0 w:1)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn commit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `790`
		//  Estimated: `11680`
		// Minimum execution time: 528_298_000 picoseconds.
		Weight::from_parts(562_322_000, 0)
			.saturating_add(Weight::from_parts(0, 11680))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Shards::ShardMembers` (r:4 w:1)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:1 w:0)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:1 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkGatewayAddress` (r:1 w:0)
	/// Proof: `Networks::NetworkGatewayAddress` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:0 w:1)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkShards` (r:0 w:1)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn ready() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `624`
		//  Estimated: `11514`
		// Minimum execution time: 64_050_000 picoseconds.
		Weight::from_parts(67_878_000, 0)
			.saturating_add(Weight::from_parts(0, 11514))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Shards::ShardNetwork` (r:1 w:1)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:1 w:0)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:1 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:4 w:3)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberRegistered` (r:3 w:0)
	/// Proof: `Members::MemberRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberStaker` (r:3 w:0)
	/// Proof: `Members::MemberStaker` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Unassigned` (r:1 w:1)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::MemberShard` (r:0 w:3)
	/// Proof: `Shards::MemberShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardThreshold` (r:0 w:1)
	/// Proof: `Shards::ShardThreshold` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:0 w:1)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberNetwork` (r:0 w:3)
	/// Proof: `Members::MemberNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPublicKey` (r:0 w:3)
	/// Proof: `Members::MemberPublicKey` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPeerId` (r:0 w:3)
	/// Proof: `Members::MemberPeerId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTaskCount` (r:0 w:1)
	/// Proof: `Tasks::ShardTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkShards` (r:0 w:1)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn force_shard_offline() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `615`
		//  Estimated: `11505`
		// Minimum execution time: 124_997_000 picoseconds.
		Weight::from_parts(132_390_000, 0)
			.saturating_add(Weight::from_parts(0, 11505))
			.saturating_add(T::DbWeight::get().reads(14))
			.saturating_add(T::DbWeight::get().writes(21))
	}
	/// Storage: `Shards::DkgTimeout` (r:103 w:102)
	/// Proof: `Shards::DkgTimeout` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:102 w:102)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:102 w:102)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:102 w:0)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:102 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:408 w:306)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberRegistered` (r:3 w:0)
	/// Proof: `Members::MemberRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberStaker` (r:3 w:0)
	/// Proof: `Members::MemberStaker` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Unassigned` (r:1 w:1)
	/// Proof: `Elections::Unassigned` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::MemberShard` (r:0 w:3)
	/// Proof: `Shards::MemberShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardThreshold` (r:0 w:102)
	/// Proof: `Shards::ShardThreshold` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::DkgTimeoutCounter` (r:0 w:1)
	/// Proof: `Shards::DkgTimeoutCounter` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberNetwork` (r:0 w:3)
	/// Proof: `Members::MemberNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPublicKey` (r:0 w:3)
	/// Proof: `Members::MemberPublicKey` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPeerId` (r:0 w:3)
	/// Proof: `Members::MemberPeerId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTaskCount` (r:0 w:102)
	/// Proof: `Tasks::ShardTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkShards` (r:0 w:102)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 102]`.
	/// The range of component `b` is `[1, 102]`.
	fn timeout_dkgs(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `377 + b * (278 ±0)`
		//  Estimated: `8789 + b * (10178 ±0)`
		// Minimum execution time: 137_520_000 picoseconds.
		Weight::from_parts(140_475_000, 0)
			.saturating_add(Weight::from_parts(0, 8789))
			// Standard Error: 388_078
			.saturating_add(Weight::from_parts(132_398_511, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((9_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(14))
			.saturating_add(T::DbWeight::get().writes((9_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 10178).saturating_mul(b.into()))
	}
}
