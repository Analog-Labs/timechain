//! Tokenomics configurations.
use smallvec::smallvec;

use polkadot_sdk::*;

use frame_support::weights::{
	constants::WEIGHT_REF_TIME_PER_SECOND, WeightToFeeCoefficient, WeightToFeeCoefficients,
	WeightToFeePolynomial,
};

#[cfg(feature = "testnet")]
use frame_support::traits::{Imbalance, OnUnbalanced};
use frame_support::{
	parameter_types,
	traits::{ConstU32, WithdrawReasons},
};

use sp_runtime::{
	traits::{Bounded, ConvertInto},
	FixedPointNumber, Perbill, Perquintill,
};

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};

// Local module imports
use crate::{
	weights, Balance, Balances, ExtrinsicBaseWeight, Runtime, RuntimeEvent, RuntimeFreezeReason,
	RuntimeHoldReason, System, ANLOG, MAX_BLOCK_LENGTH,
};
#[cfg(feature = "testnet")]
use crate::{Authorship, NegativeImbalance, Treasury};
#[cfg(feature = "testnet")]
use frame_support::traits::Currency;
use time_primitives::{MICROANLOG, MILLIANLOG};

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

pub const MIN_LINEAR_WEIGHT_FEE: Balance = MILLIANLOG;
pub const MAX_QUADRATIC_WEIGHT_FEE: Balance = 90_000 * ANLOG;

pub const MAXIMUM_BLOCK_WEIGHT_SECONDS: u64 = 2;

/// By introducing a second-degree term, the fee will grow faster as the weight increases.
/// This can be useful for discouraging transactions that consume excessive resources.
/// While a first-degree polynomial gives a linear fee increase, adding a quadratic term
/// will make fees grow non-linearly, meaning larger weights result in disproportionately larger fees.
/// This change can help manage network congestion by making resource-heavy operations more expensive.
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// Quadratic term maps max weight to max quadratic fee
		let q_2: Balance = MAX_QUADRATIC_WEIGHT_FEE * MAX_QUADRATIC_WEIGHT_FEE;
		let p_2 = WEIGHT_REF_TIME_PER_SECOND.saturating_mul(MAXIMUM_BLOCK_WEIGHT_SECONDS) as u128;
		// Linear term map linear minimum to base weight
		let p_1 = MIN_LINEAR_WEIGHT_FEE;
		let q_1 = Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![
			WeightToFeeCoefficient {
				degree: 2,
				negative: false,
				coeff_frac: Perbill::from_rational(p_2 % q_2, q_2),
				coeff_integer: p_2 / q_2,
			},
			WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p_1 % q_1, q_1),
				coeff_integer: p_1 / q_1,
			}
		]
	}
}

pub const MIN_LINEAR_LENGTH_FEE: Balance = MICROANLOG;
pub const MAX_QUADRATIC_LENGTH_FEE: Balance = 10_000 * ANLOG;

pub struct LengthToFee;
impl WeightToFeePolynomial for LengthToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// Quadratic term maps max weight to max quadratic fee
		let q_2 = MAX_QUADRATIC_LENGTH_FEE * MAX_QUADRATIC_WEIGHT_FEE;
		let p_2 = MAX_BLOCK_LENGTH as u128;
		// Linear minimum is mapped to size of smallest transaction
		let p_1 = MIN_LINEAR_LENGTH_FEE;
		let q_1 = 2;
		smallvec![
			WeightToFeeCoefficient {
				degree: 2,
				negative: false,
				coeff_frac: Perbill::from_rational(p_2 % q_2, q_2),
				coeff_integer: p_2 / q_2,
			},
			WeightToFeeCoefficient {
				degree: 1,
				negative: false,
				coeff_frac: Perbill::from_rational(p_1 % q_1, q_1),
				coeff_integer: p_1 / q_1,
			}
		]
	}
}

#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
	/// Minimum allowed account balance under which account will be reaped
	pub const ExistentialDeposit: Balance = 1 * ANLOG;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	// Use more u32 friendly value for benchmark runtime and AtLeast32Bit
	pub const ExistentialDeposit: Balance = 500;
}

pub struct Author;
#[cfg(feature = "testnet")]
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		if let Some(author) = Authorship::author() {
			Balances::resolve_creating(&author, amount);
		}
	}
}

pub struct DealWithFees;
#[cfg(feature = "testnet")]
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 80% to treasury, 20% to author
			let mut split = fees.ration(80, 20);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 80% to treasury, 20% to author (though this can be anything)
				tips.ration_merge_into(80, 20, &mut split);
			}
			#[cfg(feature = "testnet")]
			Treasury::on_unbalanced(split.0);
			Author::on_unbalanced(split.1);
		}
	}
}

parameter_types! {
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

/// ## <a id="config.Balances">[`Balances`] Config</a>
///
/// Add balance tracking and transfers
impl pallet_balances::Config for Runtime {
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<1>;
}

parameter_types! {
	/// Multiplier for operational fees, set to 5.
	pub const OperationalFeeMultiplier: u8 = 5;

	/// The target block fullness level, set to 25%.
	/// This determines the block saturation level, and fees will adjust based on this value.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);

	/// Adjustment variable for fee calculation, set to 1/100,000.
	/// This value influences how rapidly the fee multiplier changes.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);

	/// Minimum fee multiplier, set to 1/1,000,000,000.
	/// This represents the smallest possible fee multiplier to prevent fees from dropping too low.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);

	/// Maximum fee multiplier, set to the maximum possible value of the `Multiplier` type.
	pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

/// Parameterized slow adjusting fee updated based on
/// <https://research.web3.foundation/en/latest/polkadot/overview/2-token-economics.html#-2.-slow-adjusting-mechanism>
pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
	R,
	TargetBlockFullness,
	AdjustmentVariable,
	MinimumMultiplier,
	MaximumMultiplier,
>;

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
/// ## <a id="config.TransactionPayment">TransactionPayment Config</a>
///
/// Charge users for their transactions according to the transactions weight.
/// - [`WeightToFee`](#associatedtype.WeightToFee) is a custom curve, for details see `crate::tokenomics`
/// - [`LengthToFee`](#associatedtype.LengthToFee) is a custom curve, for details see `crate::tokenomics`
impl pallet_transaction_payment::Config for Runtime {
	/// The event type that will be emitted for transaction payment events.
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "testnet"))]
	/// Disabled fee distribution on mainnet
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;

	/// Specifies the currency adapter used for charging transaction fees.
	/// The `CurrencyAdapter` is used to charge the fees and deal with any adjustments or redistribution of those fees.
	#[cfg(feature = "testnet")]
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;

	/// The multiplier applied to operational transaction fees.
	/// Operational fees are used for transactions that are essential for the network's operation.
	type OperationalFeeMultiplier = OperationalFeeMultiplier;

	/// Use our custom weight to fee curve
	type WeightToFee = WeightToFee;

	/// Use our custom length to fee curve
	type LengthToFee = LengthToFee;

	/// Defines how the fee multiplier is updated based on the block fullness.
	/// The `TargetedFeeAdjustment` adjusts the fee multiplier to maintain the target block fullness.
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 1 * ANLOG;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

/// ## <a id="config.Vesting">`Vesting` Config</a>
///
/// Allow tokens to be locked following schedule
impl pallet_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	// `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
	// highest number of schedules that encodes less than 2^10.
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

#[cfg(test)]
mod test {
	use polkadot_sdk::*;
	use time_primitives::{MICROANLOG, MILLIANLOG};

	use frame_support::{
		dispatch::{DispatchClass, DispatchInfo},
		traits::OnFinalize,
		weights::{Weight, WeightToFee as WeightToFeeT},
	};
	use pallet_transaction_payment::Multiplier;
	use separator::Separatable;
	use sp_runtime::BuildStorage;
	//use pallet_elections::WeightInfo as W4;
	//
	//use pallet_members::WeightInfo as W2;
	//use pallet_tasks::WeightInfo;

	use super::{MinimumMultiplier, SlowAdjustingFeeUpdate, TargetBlockFullness};
	use crate::{
		ExtrinsicBaseWeight, Runtime, RuntimeBlockWeights, System, TransactionPayment,
		MAXIMUM_BLOCK_WEIGHT,
	};

	#[test]
	// Test that the fee for `MAXIMUM_BLOCK_WEIGHT` of weight has sane bounds.
	fn full_block_fee_is_correct() {
		// A full block should cost between 5,000 and 10,000 MILLIANLOG.
		let full_block = crate::WeightToFee::weight_to_fee(&MAXIMUM_BLOCK_WEIGHT);
		println!("FULL BLOCK Fee: {}", full_block);
		assert!(full_block >= 5_000 * MILLIANLOG);
		assert!(full_block <= 10_000 * MILLIANLOG);
	}

	#[test]
	// This function tests that the fee for `ExtrinsicBaseWeight` of weight is correct
	fn extrinsic_base_fee_is_correct() {
		// `ExtrinsicBaseWeight` should cost MICROANLOG
		println!("Base: {}", ExtrinsicBaseWeight::get());
		let x = crate::WeightToFee::weight_to_fee(&ExtrinsicBaseWeight::get());
		let y = MILLIANLOG;
		assert!(x.max(y) - x.min(y) < MICROANLOG);
	}

	fn run_with_system_weight<F>(w: Weight, mut assertions: F)
	where
		F: FnMut(),
	{
		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		t.execute_with(|| {
			System::set_block_consumed_resources(w, 0);
			assertions()
		});
	}

	#[test]
	fn multiplier_can_grow_from_zero() {
		let minimum_multiplier = MinimumMultiplier::get();
		let target = TargetBlockFullness::get()
			* RuntimeBlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		// if the min is too small, then this will not change, and we are doomed forever.
		// the weight is 1/100th bigger than target.
		run_with_system_weight(target.saturating_mul(101) / 100, || {
			use sp_runtime::traits::Convert;

			let next = SlowAdjustingFeeUpdate::<Runtime>::convert(minimum_multiplier);
			assert!(next > minimum_multiplier, "{:?} !>= {:?}", next, minimum_multiplier);
		})
	}

	#[test]
	fn multiplier_growth_simulator() {
		// assume the multiplier is initially set to its minimum. We update it with values twice the
		//target (target is 25%, thus 50%) and we see at which point it reaches 1.
		let mut multiplier = MinimumMultiplier::get();
		let block_weight = RuntimeBlockWeights::get().get(DispatchClass::Normal).max_total.unwrap();
		let mut blocks = 0;
		let mut fees_paid = 0;
		let info = DispatchInfo {
			weight: Weight::MAX,
			..Default::default()
		};

		let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap()
			.into();
		// set the minimum
		t.execute_with(|| {
			frame_system::Pallet::<Runtime>::set_block_consumed_resources(Weight::MAX, 0);
			pallet_transaction_payment::NextFeeMultiplier::<Runtime>::set(MinimumMultiplier::get());
		});

		while multiplier <= Multiplier::from_u32(1) {
			t.execute_with(|| {
				// imagine this tx was called.
				let fee = TransactionPayment::compute_fee(0, &info, 0);
				fees_paid += fee;

				// this will update the multiplier.
				System::set_block_consumed_resources(block_weight, 0);
				TransactionPayment::on_finalize(1);
				let next = TransactionPayment::next_fee_multiplier();

				assert!(next > multiplier, "{:?} !>= {:?}", next, multiplier);
				multiplier = next;

				println!(
					"block = {} / multiplier {:?} / fee = {:?} / fess so far {:?}",
					blocks,
					multiplier,
					fee.separated_string(),
					fees_paid.separated_string()
				);
			});
			blocks += 1;
		}
	}
}
