//! Common types and constants used in both runtimes and any pallet
#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_sdk::*;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
#[cfg(any(feature = "std", test))]
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use pallet_utility::Call as UtilityCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Weight matters.
pub mod weights;

use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};

/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

/// Money matters.
pub mod currency {
	pub use time_primitives::currency::*;

	pub const TOTAL_ISSUANCE: Balance = 90_570_710 * ANLOG;

	pub const TRANSACTION_BYTE_FEE: Balance = MICROANLOG;
	pub const STORAGE_BYTE_FEE: Balance = 300 * MILLIANLOG;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 750 * MILLIANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
	}
}

/// Time.
pub mod time {
	use time_primitives::{BlockNumber, Moment};

	/// Average expected block time that we are targeting.
	pub const MILLISECS_PER_BLOCK: Moment = 6000;

	/// Minimum duration at which blocks will be produced.
	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// These time units are defined in number of blocks.
	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
	pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;

	pub const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

	/// TODO: 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);
}

/// Fee-Calculation.
pub mod fee {
	use super::frame_support;
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
}

/// Shared signing extensions
pub type SignedExtra<Runtime> = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

use time_primitives::{AccountId, Signature};

/// The address format for describing accounts.
type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic<Runtime, Call> =
	sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra<Runtime>>;
/// The payload being signed in transactions.
pub type SignedPayload<Runtime, Call> =
	sp_runtime::generic::SignedPayload<Call, SignedExtra<Runtime>>;

/// Shared default babe genesis config
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: time::PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// Macro to set a value (e.g. when using the `parameter_types` macro) to either
/// a production value or a development value (in case the `development` feature is
/// selected).
///
/// Usage:
/// ```Rust
/// parameter_types! {
///     pub const VotingPeriod: BlockNumber = prod_or_dev!(7 * DAYS, 1 * MINUTES);
/// }
/// ```
#[macro_export]
macro_rules! prod_or_dev {
	($prod:expr, $fast:expr) => {
		if cfg!(feature = "development") {
			$fast
		} else {
			$prod
		}
	};
}

#[cfg(test)]
mod tests {
	use super::{
		currency::*, fee::WeightToFee, frame_support::weights::WeightToFee as WeightToFeeT,
		MAXIMUM_BLOCK_WEIGHT,
	};
	use crate::weights::ExtrinsicBaseWeight;

	#[test]
	// Test that the fee for `MAXIMUM_BLOCK_WEIGHT` of weight has sane bounds.
	fn full_block_fee_is_correct() {
		// A full block should cost between 5,000 and 10,000 MILLIANLOG.
		let full_block = WeightToFee::weight_to_fee(&MAXIMUM_BLOCK_WEIGHT);
		println!("FULL BLOCK Fee: {}", full_block);
		assert!(full_block >= 5_000 * MILLIANLOG);
		assert!(full_block <= 10_000 * MILLIANLOG);
	}

	#[test]
	// This function tests that the fee for `ExtrinsicBaseWeight` of weight is correct
	fn extrinsic_base_fee_is_correct() {
		// `ExtrinsicBaseWeight` should cost MICROANLOG
		println!("Base: {}", ExtrinsicBaseWeight::get());
		let x = WeightToFee::weight_to_fee(&ExtrinsicBaseWeight::get());
		let y = MILLIANLOG;
		assert!(x.max(y) - x.min(y) < MICROANLOG);
	}
}
