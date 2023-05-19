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
