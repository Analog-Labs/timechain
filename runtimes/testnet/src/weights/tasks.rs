
//! Autogenerated weights for `pallet_tasks`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-10-24, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet_tasks
// --extrinsic
// *
// --output
// ./weights/tasks.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_tasks`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_tasks::WeightInfo for WeightInfo<T> {
	/// Storage: `Tasks::Tasks` (r:1 w:1)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskOutput` (r:1 w:1)
	/// Proof: `Tasks::TaskOutput` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:1 w:1)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardNetwork` (r:1 w:0)
	/// Proof: `Shards::ShardNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:1 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchSize` (r:1 w:0)
	/// Proof: `Networks::NetworkBatchSize` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchOffset` (r:1 w:0)
	/// Proof: `Networks::NetworkBatchOffset` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskIdCounter` (r:1 w:1)
	/// Proof: `Tasks::TaskIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskCount` (r:1 w:1)
	/// Proof: `Tasks::TaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardMembers` (r:4 w:0)
	/// Proof: `Shards::ShardMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:4 w:4)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Tasks::ShardTaskCount` (r:1 w:1)
	/// Proof: `Tasks::ShardTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ExecutedTaskCount` (r:1 w:1)
	/// Proof: `Tasks::ExecutedTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:0 w:1)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ReadEventsTask` (r:0 w:1)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskNetwork` (r:0 w:1)
	/// Proof: `Tasks::TaskNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn submit_task_result() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1499`
		//  Estimated: `12389`
		// Minimum execution time: 641_250_000 picoseconds.
		Weight::from_parts(667_368_000, 0)
			.saturating_add(Weight::from_parts(0, 12389))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(14))
	}
	/// Storage: `Tasks::ReadEventsTask` (r:501 w:0)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkShardTaskLimit` (r:500 w:0)
	/// Proof: `Networks::NetworkShardTaskLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:500 w:500)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkShards` (r:1000 w:0)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTaskCount` (r:500 w:500)
	/// Proof: `Tasks::ShardTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:500 w:0)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:500 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardRegistered` (r:1 w:0)
	/// Proof: `Tasks::ShardRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskCount` (r:500 w:0)
	/// Proof: `Tasks::TaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ExecutedTaskCount` (r:500 w:0)
	/// Proof: `Tasks::ExecutedTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:0 w:500)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 500]`.
	/// The range of component `b` is `[1, 500]`.
	fn schedule_tasks(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1011 + b * (219 ±0)`
		//  Estimated: `4464 + b * (5169 ±0)`
		// Minimum execution time: 76_353_000 picoseconds.
		Weight::from_parts(78_848_000, 0)
			.saturating_add(Weight::from_parts(0, 4464))
			// Standard Error: 114_956
			.saturating_add(Weight::from_parts(73_632_341, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((10_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 5169).saturating_mul(b.into()))
	}
	/// Storage: `Tasks::ReadEventsTask` (r:1000 w:0)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchGasLimit` (r:999 w:0)
	/// Proof: `Networks::NetworkBatchGasLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::OpsRemoveIndex` (r:999 w:999)
	/// Proof: `Tasks::OpsRemoveIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::OpsInsertIndex` (r:999 w:0)
	/// Proof: `Tasks::OpsInsertIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Ops` (r:1998 w:999)
	/// Proof: `Tasks::Ops` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::BatchIdCounter` (r:1 w:1)
	/// Proof: `Tasks::BatchIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskIdCounter` (r:1 w:1)
	/// Proof: `Tasks::TaskIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UATasksInsertIndex` (r:999 w:999)
	/// Proof: `Tasks::UATasksInsertIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskCount` (r:999 w:999)
	/// Proof: `Tasks::TaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::BatchTaskId` (r:0 w:999)
	/// Proof: `Tasks::BatchTaskId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UATasks` (r:0 w:999)
	/// Proof: `Tasks::UATasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::BatchMessage` (r:0 w:999)
	/// Proof: `Tasks::BatchMessage` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:0 w:999)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskNetwork` (r:0 w:999)
	/// Proof: `Tasks::TaskNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 1000]`.
	/// The range of component `b` is `[1, 1000]`.
	fn prepare_batches(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1052 + b * (173 ±0)`
		//  Estimated: `4498 + b * (5124 ±0)`
		// Minimum execution time: 76_523_000 picoseconds.
		Weight::from_parts(78_657_000, 0)
			.saturating_add(Weight::from_parts(0, 4498))
			// Standard Error: 61_273
			.saturating_add(Weight::from_parts(68_360_305, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((8_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((9_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 5124).saturating_mul(b.into()))
	}

	fn submit_gmp_events() -> Weight {
		Weight::default()
	}

	fn stop_network() -> Weight {
		Weight::default()
	}

	fn sync_network() -> Weight {
		Weight::default()
	}

	fn remove_task() -> Weight {
		Weight::default()
	}
}
