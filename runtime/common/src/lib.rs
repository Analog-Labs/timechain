//! common types and constants used in both pallet tests and runtime
#![cfg_attr(not(feature = "std"), no_std)]

pub mod weights;

/// Money matters
pub mod currency {

	pub type Balance = u128;

	pub const TOKEN_DECIMALS: u32 = 8;
	const TOKEN_BASE: u128 = 10;
	pub const ANLOG: Balance = TOKEN_BASE.pow(TOKEN_DECIMALS);
	pub const MILLIANLOG: Balance = ANLOG / 1000;
	pub const MICROANLOG: Balance = MILLIANLOG / 1000;
	pub const TOCK: Balance = MICROANLOG / 100;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 15 * MICROANLOG + (bytes as Balance) * 6 * MICROANLOG
	}
}

/// Time and blocks.
pub mod time {
	// TODO: move types into common primitives crate at root level
	pub type BlockNumber = u32;
	pub type Moment = u64;
	pub const MILLISECS_PER_BLOCK: Moment = 6000;
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// check current value
	pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = 1 * HOURS;

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
	pub const WEEKS: BlockNumber = DAYS * 7;

	// 1 in 4 blocks (on average, not counting collisions) will be primary babe blocks.
	// The choice of is done in accordance to the slot duration and expected target
	// block time, for safely resisting network delays of maximum two seconds.
	// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

/// Fee-related.
pub mod fee {
	use crate::weights::ExtrinsicBaseWeight;
	use frame_support::weights::{
		WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
	};
	use smallvec::smallvec;
	pub use sp_runtime::Perbill;
	// TODO: move into root level primitives
	pub type Balance = u128;

	/// The block saturation level. Fees will be updates based on this value.
	pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

	/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
	/// node's balance type.
	///
	/// This should typically create a mapping between the following ranges:
	///   - [0, `MAXIMUM_BLOCK_WEIGHT`]
	///   - [Balance::min, Balance::max]
	///
	/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
	///   - Setting it to `0` will essentially disable the weight fee.
	///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
	pub struct WeightToFee;
	impl WeightToFeePolynomial for WeightToFee {
		type Balance = Balance;
		fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
			// in Kusama, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
			let p = super::currency::MICROANLOG;
			let q = 10 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
			smallvec![WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}]
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{
		currency::{MICROANLOG, MILLIANLOG},
		fee::WeightToFee,
	};
	use crate::weights::ExtrinsicBaseWeight;
	use frame_support::weights::WeightToFee as WeightToFeeT;
	use runtime_common::MAXIMUM_BLOCK_WEIGHT;

	#[test]
	// Test that the fee for `MAXIMUM_BLOCK_WEIGHT` of weight has sane bounds.
	fn full_block_fee_is_correct() {
		// A full block should cost between 1,000 and 10,000 CENTS.
		let full_block = WeightToFee::weight_to_fee(&MAXIMUM_BLOCK_WEIGHT);
		assert!(full_block >= 1_000 * MICROANLOG);
		assert!(full_block <= 10_000 * MICROANLOG);
	}

	#[test]
	// This function tests that the fee for `ExtrinsicBaseWeight` of weight is correct
	fn extrinsic_base_fee_is_correct() {
		// `ExtrinsicBaseWeight` should cost 1/10 of a MICROANOG
		println!("Base: {}", ExtrinsicBaseWeight::get());
		let x = WeightToFee::weight_to_fee(&ExtrinsicBaseWeight::get());
		let y = MICROANLOG / 10;
		assert!(x.max(y) - x.min(y) < MILLIANLOG);
	}
}
