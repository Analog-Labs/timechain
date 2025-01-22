
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.1
//! DATE: 2025-01-22 (Y/M/D)
//! HOSTNAME: `station`, CPU: `AMD Ryzen Threadripper PRO 5945WX 12-Cores`
//!
//! SHORT-NAME: `block`, LONG-NAME: `BlockExecution`, RUNTIME: `Analog Testnet Local`
//! WARMUPS: `10`, REPEAT: `100`
//! WEIGHT-PATH: `runtime/src/weights/testnet/`
//! WEIGHT-METRIC: `Average`, WEIGHT-MUL: `1.0`, WEIGHT-ADD: `0`

// Executed Command:
//   target/release/timechain-node
//   benchmark
//   overhead
//   --dev
//   --wasm-execution=compiled
//   --weight-path
//   runtime/src/weights/testnet/

use sp_core::parameter_types;
use sp_weights::{constants::WEIGHT_REF_TIME_PER_NANOS, Weight};

parameter_types! {
	/// Time to execute an empty block.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 264_567, 352_513
	///   Average:  280_089
	///   Median:   276_370
	///   Std-Dev:  14162.34
	///
	/// Percentiles nanoseconds:
	///   99th: 349_277
	///   95th: 296_858
	///   75th: 286_769
	pub const BlockExecutionWeight: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(280_089), 0);
}

#[cfg(test)]
mod test_weights {
	use sp_weights::constants;

	/// Checks that the weight exists and is sane.
	// NOTE: If this test fails but you are sure that the generated values are fine,
	// you can delete it.
	#[test]
	fn sane() {
		let w = super::BlockExecutionWeight::get();

		// At least 100 µs.
		assert!(
			w.ref_time() >= 100u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
			"Weight should be at least 100 µs."
		);
		// At most 50 ms.
		assert!(
			w.ref_time() <= 50u64 * constants::WEIGHT_REF_TIME_PER_MILLIS,
			"Weight should be at most 50 ms."
		);
	}
}
