
//! Autogenerated weights for `pallet_tasks`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-04-25, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `benchmark-agent-1`, CPU: `AMD EPYC Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// ./target/release/timechain-node
// benchmark
// pallet
// --pallet
// pallet_tasks
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --output
// ./runtime/src/weights/tasks.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_tasks`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_tasks::WeightInfo for WeightInfo<T> {
	/// Storage: `Shards::ShardNetwork` (r:2 w:0)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardState` (r:1 w:0)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:4 w:0)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskIdCounter` (r:1 w:1)
	/// Proof: `Tasks::TaskIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkReadReward` (r:1 w:0)
	/// Proof: `Tasks::NetworkReadReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkWriteReward` (r:1 w:0)
	/// Proof: `Tasks::NetworkWriteReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkSendMessageReward` (r:1 w:0)
	/// Proof: `Tasks::NetworkSendMessageReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Tasks::NetworkShards` (r:2 w:0)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:1 w:1)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardRegistered` (r:1 w:0)
	/// Proof: `Tasks::ShardRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UnassignedTasks` (r:2 w:1)
	/// Proof: `Tasks::UnassignedTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:1 w:1)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskRewardConfig` (r:0 w:1)
	/// Proof: `Tasks::TaskRewardConfig` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::PhaseStart` (r:0 w:1)
	/// Proof: `Tasks::PhaseStart` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskPhaseState` (r:0 w:1)
	/// Proof: `Tasks::TaskPhaseState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:0 w:1)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 10000]`.
	/// The range of component `b` is `[1, 10000]`.
	fn create_task(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `599`
		//  Estimated: `11489`
		// Minimum execution time: 212_516_000 picoseconds.
		Weight::from_parts(221_250_597, 0)
			.saturating_add(Weight::from_parts(0, 11489))
			// Standard Error: 58
			.saturating_add(Weight::from_parts(1_299, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Tasks::Tasks` (r:1 w:0)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskOutput` (r:1 w:1)
	/// Proof: `Tasks::TaskOutput` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskPhaseState` (r:1 w:0)
	/// Proof: `Tasks::TaskPhaseState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:1 w:1)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:1 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::PhaseStart` (r:1 w:1)
	/// Proof: `Tasks::PhaseStart` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskRewardConfig` (r:1 w:1)
	/// Proof: `Tasks::TaskRewardConfig` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:4 w:0)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:4 w:4)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Tasks::SignerPayout` (r:1 w:0)
	/// Proof: `Tasks::SignerPayout` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:2 w:1)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardRegistered` (r:1 w:0)
	/// Proof: `Tasks::ShardRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UnassignedTasks` (r:1 w:0)
	/// Proof: `Tasks::UnassignedTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn submit_result(_: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1226`
		//  Estimated: `12116`
		// Minimum execution time: 788_400_000 picoseconds.
		Weight::from_parts(814_147_000, 0)
			.saturating_add(Weight::from_parts(0, 12116))
			.saturating_add(T::DbWeight::get().reads(20))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Tasks::Tasks` (r:1 w:0)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskPhaseState` (r:1 w:1)
	/// Proof: `Tasks::TaskPhaseState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskSigner` (r:1 w:0)
	/// Proof: `Tasks::TaskSigner` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:1 w:0)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskRewardConfig` (r:1 w:0)
	/// Proof: `Tasks::TaskRewardConfig` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::PhaseStart` (r:1 w:2)
	/// Proof: `Tasks::PhaseStart` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskHash` (r:0 w:1)
	/// Proof: `Tasks::TaskHash` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::SignerPayout` (r:0 w:1)
	/// Proof: `Tasks::SignerPayout` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn submit_hash() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `621`
		//  Estimated: `4086`
		// Minimum execution time: 55_243_000 picoseconds.
		Weight::from_parts(57_086_000, 0)
			.saturating_add(Weight::from_parts(0, 4086))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Tasks::TaskSignature` (r:1 w:1)
	/// Proof: `Tasks::TaskSignature` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:1 w:0)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskPhaseState` (r:1 w:1)
	/// Proof: `Tasks::TaskPhaseState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:1 w:0)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:1 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:1 w:0)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Gateway` (r:1 w:0)
	/// Proof: `Tasks::Gateway` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:4 w:0)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::SignerIndex` (r:1 w:1)
	/// Proof: `Shards::SignerIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Members::MemberPublicKey` (r:1 w:0)
	/// Proof: `Members::MemberPublicKey` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::PhaseStart` (r:0 w:1)
	/// Proof: `Tasks::PhaseStart` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskSigner` (r:0 w:1)
	/// Proof: `Tasks::TaskSigner` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn submit_signature() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1653`
		//  Estimated: `12543`
		// Minimum execution time: 586_763_000 picoseconds.
		Weight::from_parts(609_807_000, 0)
			.saturating_add(Weight::from_parts(0, 12543))
			.saturating_add(T::DbWeight::get().reads(13))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Shards::ShardState` (r:1 w:0)
	/// Proof: `Shards::ShardState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:1 w:0)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkShards` (r:2 w:0)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardRegistered` (r:1 w:1)
	/// Proof: `Tasks::ShardRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:1 w:1)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Gateway` (r:1 w:1)
	/// Proof: `Tasks::Gateway` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::ShardSize` (r:1 w:0)
	/// Proof: `Elections::ShardSize` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskIdCounter` (r:1 w:1)
	/// Proof: `Tasks::TaskIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkReadReward` (r:1 w:0)
	/// Proof: `Tasks::NetworkReadReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkWriteReward` (r:1 w:0)
	/// Proof: `Tasks::NetworkWriteReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkSendMessageReward` (r:1 w:0)
	/// Proof: `Tasks::NetworkSendMessageReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Tasks::ShardTasks` (r:2 w:1)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:4 w:0)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UnassignedTasks` (r:2 w:1)
	/// Proof: `Tasks::UnassignedTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:1 w:1)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskRewardConfig` (r:0 w:1)
	/// Proof: `Tasks::TaskRewardConfig` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::RecvTasks` (r:0 w:1)
	/// Proof: `Tasks::RecvTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::PhaseStart` (r:0 w:1)
	/// Proof: `Tasks::PhaseStart` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskPhaseState` (r:0 w:1)
	/// Proof: `Tasks::TaskPhaseState` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn register_gateway() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `722`
		//  Estimated: `11612`
		// Minimum execution time: 240_981_000 picoseconds.
		Weight::from_parts(251_880_000, 0)
			.saturating_add(Weight::from_parts(0, 11612))
			.saturating_add(T::DbWeight::get().reads(22))
			.saturating_add(T::DbWeight::get().writes(12))
	}
	/// Storage: `Tasks::NetworkReadReward` (r:0 w:1)
	/// Proof: `Tasks::NetworkReadReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn set_read_task_reward() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 11_071_000 picoseconds.
		Weight::from_parts(11_953_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Tasks::NetworkWriteReward` (r:0 w:1)
	/// Proof: `Tasks::NetworkWriteReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn set_write_task_reward() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 10_970_000 picoseconds.
		Weight::from_parts(11_943_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Tasks::NetworkSendMessageReward` (r:0 w:1)
	/// Proof: `Tasks::NetworkSendMessageReward` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn set_send_message_task_reward() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 10_949_000 picoseconds.
		Weight::from_parts(12_464_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	fn cancel_task() -> Weight {
		Weight::default()
	}
	fn reset_tasks() -> Weight {
		Weight::default()
	}

	fn unregister_gateways() -> Weight {
		Weight::from_parts(0, 0)
	}

	fn set_shard_task_limit() -> Weight {
		Weight::from_parts(0, 0)
	}

	fn set_batch_size() -> Weight {
		Weight::from_parts(0, 0)
	}
}
