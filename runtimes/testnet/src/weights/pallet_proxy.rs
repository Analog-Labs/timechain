
//! Autogenerated weights for `pallet_proxy`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.0
//! DATE: 2024-10-31, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet_proxy
// --extrinsic
// *
// --output
// ./weights/pallet_proxy.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use polkadot_sdk::*;

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_proxy`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_proxy::WeightInfo for WeightInfo<T> {
	/// Storage: `Proxy::Proxies` (r:1 w:0)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn proxy(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 18_704_000 picoseconds.
		Weight::from_parts(22_241_983, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 17_618
			.saturating_add(Weight::from_parts(34_410, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:0)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn proxy_announced(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `459 + a * (68 ±0) + p * (36 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 48_861_000 picoseconds.
		Weight::from_parts(51_972_349, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 27_388
			.saturating_add(Weight::from_parts(332_768, 0).saturating_mul(a.into()))
			// Standard Error: 28_297
			.saturating_add(Weight::from_parts(55_130, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn remove_announcement(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `374 + a * (68 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 32_991_000 picoseconds.
		Weight::from_parts(35_817_302, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 14_781
			.saturating_add(Weight::from_parts(309_227, 0).saturating_mul(a.into()))
			// Standard Error: 15_271
			.saturating_add(Weight::from_parts(7_053, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn reject_announcement(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `374 + a * (68 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 33_363_000 picoseconds.
		Weight::from_parts(36_948_386, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 28_770
			.saturating_add(Weight::from_parts(231_760, 0).saturating_mul(a.into()))
			// Standard Error: 29_725
			.saturating_add(Weight::from_parts(45_083, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:0)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn announce(a: u32, _p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `391 + a * (68 ±0) + p * (36 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 43_822_000 picoseconds.
		Weight::from_parts(46_629_046, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 25_509
			.saturating_add(Weight::from_parts(313_981, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn add_proxy(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 31_237_000 picoseconds.
		Weight::from_parts(35_942_624, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 46_657
			.saturating_add(Weight::from_parts(219_973, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn remove_proxy(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 31_890_000 picoseconds.
		Weight::from_parts(33_208_922, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 16_532
			.saturating_add(Weight::from_parts(211_130, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn remove_proxies(_p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 28_553_000 picoseconds.
		Weight::from_parts(34_828_566, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn create_pure(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `139`
		//  Estimated: `4706`
		// Minimum execution time: 32_842_000 picoseconds.
		Weight::from_parts(37_516_934, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 21_126
			.saturating_add(Weight::from_parts(12_648, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[0, 30]`.
	fn kill_pure(_p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `164 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 29_585_000 picoseconds.
		Weight::from_parts(38_303_913, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
