
//! Autogenerated weights for `pallet_tasks`
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
	/// Storage: `Tasks::SyncHeight` (r:1 w:1)
	/// Proof: `Tasks::SyncHeight` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ReadEventsTask` (r:1 w:1)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
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
	/// Storage: `Tasks::TaskNetwork` (r:0 w:1)
	/// Proof: `Tasks::TaskNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn submit_task_result() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1511`
		//  Estimated: `12401`
		// Minimum execution time: 644_947_000 picoseconds.
		Weight::from_parts(726_660_000, 0)
			.saturating_add(Weight::from_parts(0, 12401))
			.saturating_add(T::DbWeight::get().reads(21))
			.saturating_add(T::DbWeight::get().writes(15))
	}
	/// Storage: `Tasks::ReadEventsTask` (r:51 w:0)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkShardTaskLimit` (r:50 w:0)
	/// Proof: `Networks::NetworkShardTaskLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskShard` (r:50 w:50)
	/// Proof: `Tasks::TaskShard` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::NetworkShards` (r:100 w:0)
	/// Proof: `Tasks::NetworkShards` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTaskCount` (r:50 w:50)
	/// Proof: `Tasks::ShardTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:50 w:0)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Shards::ShardCommitment` (r:50 w:0)
	/// Proof: `Shards::ShardCommitment` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardRegistered` (r:1 w:0)
	/// Proof: `Tasks::ShardRegistered` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskCount` (r:50 w:0)
	/// Proof: `Tasks::TaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ExecutedTaskCount` (r:50 w:0)
	/// Proof: `Tasks::ExecutedTaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ShardTasks` (r:0 w:50)
	/// Proof: `Tasks::ShardTasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 50]`.
	/// The range of component `b` is `[1, 50]`.
	fn schedule_tasks(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `700 + b * (227 ±0)`
		//  Estimated: `4183 + b * (5176 ±0)`
		// Minimum execution time: 74_400_000 picoseconds.
		Weight::from_parts(80_984_904, 0)
			.saturating_add(Weight::from_parts(0, 4183))
			// Standard Error: 489_624
			.saturating_add(Weight::from_parts(65_787_073, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((10_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 5176).saturating_mul(b.into()))
	}
	/// Storage: `Tasks::ReadEventsTask` (r:11 w:0)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchGasLimit` (r:10 w:0)
	/// Proof: `Networks::NetworkBatchGasLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::OpsRemoveIndex` (r:10 w:10)
	/// Proof: `Tasks::OpsRemoveIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::OpsInsertIndex` (r:10 w:0)
	/// Proof: `Tasks::OpsInsertIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Ops` (r:20 w:10)
	/// Proof: `Tasks::Ops` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::BatchIdCounter` (r:1 w:1)
	/// Proof: `Tasks::BatchIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskIdCounter` (r:1 w:1)
	/// Proof: `Tasks::TaskIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UATasksInsertIndex` (r:10 w:10)
	/// Proof: `Tasks::UATasksInsertIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskCount` (r:10 w:10)
	/// Proof: `Tasks::TaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::BatchTaskId` (r:0 w:10)
	/// Proof: `Tasks::BatchTaskId` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::UATasks` (r:0 w:10)
	/// Proof: `Tasks::UATasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::BatchMessage` (r:0 w:10)
	/// Proof: `Tasks::BatchMessage` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:0 w:10)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskNetwork` (r:0 w:10)
	/// Proof: `Tasks::TaskNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 10]`.
	/// The range of component `b` is `[1, 10]`.
	fn prepare_batches(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `440 + b * (186 ±0)`
		//  Estimated: `3894 + b * (5137 ±0)`
		// Minimum execution time: 68_829_000 picoseconds.
		Weight::from_parts(73_758_000, 0)
			.saturating_add(Weight::from_parts(0, 3894))
			// Standard Error: 332_513
			.saturating_add(Weight::from_parts(58_203_546, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((8_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((9_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 5137).saturating_mul(b.into()))
	}
	fn submit_gmp_events() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 3_246_000 picoseconds.
		Weight::from_parts(3_907_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `Tasks::SyncHeight` (r:0 w:1)
	/// Proof: `Tasks::SyncHeight` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn sync_network() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_651_000 picoseconds.
		Weight::from_parts(6_844_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Tasks::ReadEventsTask` (r:0 w:1)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn stop_network() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_580_000 picoseconds.
		Weight::from_parts(6_893_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Tasks::TaskOutput` (r:1 w:1)
	/// Proof: `Tasks::TaskOutput` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:1 w:1)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskSubmitter` (r:0 w:1)
	/// Proof: `Tasks::TaskSubmitter` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskNetwork` (r:0 w:1)
	/// Proof: `Tasks::TaskNetwork` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn remove_task() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `286`
		//  Estimated: `3751`
		// Minimum execution time: 19_997_000 picoseconds.
		Weight::from_parts(21_620_000, 0)
			.saturating_add(Weight::from_parts(0, 3751))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(4))
	}
}
