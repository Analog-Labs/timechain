//! Fee-Calculation.

use polkadot_sdk::*;

use crate::weights::ExtrinsicBaseWeight;
use frame_support::weights::{
	WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};

use crate::currency::*;
pub use crate::sp_runtime::Perbill;
use smallvec::smallvec;

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - [0, `frame_system::MaximumBlockWeight`]
///   - [Balance::min, Balance::max]
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
/// By introducing a second-degree term, the fee will grow faster as the weight increases.
/// This can be useful for discouraging transactions that consume excessive resources.
/// While a first-degree polynomial gives a linear fee increase, adding a quadratic term
/// will make fees grow non-linearly, meaning larger weights result in disproportionately larger fees.
/// This change can help manage network congestion by making resource-heavy operations more expensive.
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Timechain, extrinsic base weight (smallest non-zero weight) is mapped to MILLIANLOG:
		let p = super::currency::MILLIANLOG;
		let q = Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![
			WeightToFeeCoefficient {
				degree: 2,
				negative: false,
				coeff_frac: Perbill::from_rational((TOCK / MILLIANLOG) % q, q),
				coeff_integer: (TOCK / MILLIANLOG) / q,
			},
			WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p % q, q),
				coeff_integer: p / q,
			}
		]
	}
}
