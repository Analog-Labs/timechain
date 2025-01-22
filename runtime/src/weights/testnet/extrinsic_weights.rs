
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 42.0.1
//! DATE: 2025-01-22 (Y/M/D)
//! HOSTNAME: `station`, CPU: `AMD Ryzen Threadripper PRO 5945WX 12-Cores`
//!
//! SHORT-NAME: `extrinsic`, LONG-NAME: `ExtrinsicBase`, RUNTIME: `Analog Testnet Local`
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
	/// Time to execute a NO-OP extrinsic, for example `System::remark`.
	/// Calculated by multiplying the *Average* with `1.0` and adding `0`.
	///
	/// Stats nanoseconds:
	///   Min, Max: 92_619, 100_897
	///   Average:  93_837
	///   Median:   93_247
	///   Std-Dev:  1525.55
	///
	/// Percentiles nanoseconds:
	///   99th: 98_443
	///   95th: 97_655
	///   75th: 93_800
	pub const ExtrinsicBaseWeight: Weight =
		Weight::from_parts(WEIGHT_REF_TIME_PER_NANOS.saturating_mul(93_837), 0);
}

#[cfg(test)]
mod test_weights {
	use sp_weights::constants;

	/// Checks that the weight exists and is sane.
	// NOTE: If this test fails but you are sure that the generated values are fine,
	// you can delete it.
	#[test]
	fn sane() {
		let w = super::ExtrinsicBaseWeight::get();

		// At least 10 µs.
		assert!(
			w.ref_time() >= 10u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
			"Weight should be at least 10 µs."
		);
		// At most 1 ms.
		assert!(
			w.ref_time() <= constants::WEIGHT_REF_TIME_PER_MILLIS,
			"Weight should be at most 1 ms."
		);
	}
}
