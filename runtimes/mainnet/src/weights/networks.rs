
//! Autogenerated weights for `pallet_networks`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-10-02, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet_networks
// --extrinsic
// *
// --output
// ./weights/networks.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_networks`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_networks::WeightInfo for WeightInfo<T> {
	/// Storage: `Networks::Networks` (r:1 w:1)
	/// Proof: `Networks::Networks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchSize` (r:1 w:1)
	/// Proof: `Networks::NetworkBatchSize` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchOffset` (r:1 w:1)
	/// Proof: `Networks::NetworkBatchOffset` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskIdCounter` (r:1 w:1)
	/// Proof: `Tasks::TaskIdCounter` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::TaskCount` (r:1 w:1)
	/// Proof: `Tasks::TaskCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkName` (r:0 w:1)
	/// Proof: `Networks::NetworkName` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkShardTaskLimit` (r:0 w:1)
	/// Proof: `Networks::NetworkShardTaskLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkGatewayAddress` (r:0 w:1)
	/// Proof: `Networks::NetworkGatewayAddress` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkGatewayBlock` (r:0 w:1)
	/// Proof: `Networks::NetworkGatewayBlock` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchGasLimit` (r:0 w:1)
	/// Proof: `Networks::NetworkBatchGasLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::Tasks` (r:0 w:1)
	/// Proof: `Tasks::Tasks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Tasks::ReadEventsTask` (r:0 w:1)
	/// Proof: `Tasks::ReadEventsTask` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `a` is `[1, 1000]`.
	/// The range of component `b` is `[1, 1000]`.
	/// The range of component `a` is `[1, 1000]`.
	/// The range of component `b` is `[1, 1000]`.
	fn register_network(a: u32, _b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `118`
		//  Estimated: `3583`
		// Minimum execution time: 55_063_000 picoseconds.
		Weight::from_parts(63_303_978, 0)
			.saturating_add(Weight::from_parts(0, 3583))
			// Standard Error: 771
			.saturating_add(Weight::from_parts(5_124, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(12))
	}
	/// Storage: `Networks::Networks` (r:1 w:0)
	/// Proof: `Networks::Networks` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchOffset` (r:0 w:1)
	/// Proof: `Networks::NetworkBatchOffset` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchSize` (r:0 w:1)
	/// Proof: `Networks::NetworkBatchSize` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkShardTaskLimit` (r:0 w:1)
	/// Proof: `Networks::NetworkShardTaskLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Networks::NetworkBatchGasLimit` (r:0 w:1)
	/// Proof: `Networks::NetworkBatchGasLimit` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn set_network_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `277`
		//  Estimated: `3742`
		// Minimum execution time: 21_800_000 picoseconds.
		Weight::from_parts(23_825_000, 0)
			.saturating_add(Weight::from_parts(0, 3742))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(4))
	}
}
