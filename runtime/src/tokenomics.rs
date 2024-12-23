//! Tokenomics configurations.


use smallvec::smallvec;
use scale_codec::{Decode, Encode, MaxEncodedLen};

use polkadot_sdk::*;

use frame_support::weights::{
	WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};

use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	pallet_prelude::Get,
	parameter_types,
	traits::{
		fungible::HoldConsideration,
		tokens::{PayFromAccount, UnityAssetBalanceConversion},
		ConstU128, ConstU16, ConstU32, Contains, Currency, EitherOfDiverse, EqualPrivilegeOnly,
		Imbalance, InstanceFilter, KeyOwnerProofSystem, LinearStoragePrice, OnUnbalanced,
		WithdrawReasons,
	},
	weights::{
		constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
		ConstantMultiplier, Weight,
	},
	PalletId,
};

use sp_runtime::{
	generic::Era,
	create_runtime_str,
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, Bounded, ConvertInto, Extrinsic, Identity as IdentityT,
		IdentityLookup, MaybeConvert, NumberFor, OpaqueKeys, SaturatedConversion, StaticLookup,
		Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedPointNumber, Perbill, Percent, Permill, Perquintill, RuntimeDebug,
};

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};

// Local module imports
use super::{
	AuthorityDiscovery,
	AccountId, Babe, Balance, Balances, Block, EpochDuration, Executive, Grandpa, Historical, InherentDataExt, Nonce, Runtime,
	RuntimeCall, RuntimeGenesisConfig, SessionKeys, System, TransactionPayment, BABE_GENESIS_EPOCH_CONFIG, VERSION,
	Treasury, RuntimeHoldReason, RuntimeFreezeReason, RuntimeEvent
};
use super::{
	weights::{self, ExtrinsicBaseWeight},
	currency::*,
};

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


#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
	/// Minimum allowed account balance under which account will be reaped
	pub const ExistentialDeposit: Balance = 1 * ANLOG;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	// Use more u32 friendly value for benchmark runtime and AtLeast32Bit
	pub const ExistentialDeposit: Balance = 500 * MILLIANLOG;
}

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		#[cfg(feature = "testnet")]
		if let Some(author) = Authorship::author() {
			Balances::resolve_creating(&author, amount);
		}
	}
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 80% to treasury, 20% to author
			let mut split = fees.ration(80, 20);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 80% to treasury, 20% to author (though this can be anything)
				tips.ration_merge_into(80, 20, &mut split);
			}
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

/// ## 06 - <a id="config.Balances">[`Balances`] Config</a>
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
/// ## 07 - <a id="config.TransactionPayment">[`TransactionPayment`] Config</a>
///
/// Charge users for their transactions according to the transactions weight.
/// - [`WeightToFee`](#associatedtype.WeightToFee) is a custom curve, for details see [`crate::fee`]
/// - [`LengtToFee`](#associatedtype.LengthToFee) is a custom curve, for details see [`crate::fee`]
impl pallet_transaction_payment::Config for Runtime {
	/// The event type that will be emitted for transaction payment events.
	type RuntimeEvent = RuntimeEvent;

	/// Specifies the currency adapter used for charging transaction fees.
	/// The `CurrencyAdapter` is used to charge the fees and deal with any adjustments or redistribution of those fees.
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;

	/// The multiplier applied to operational transaction fees.
	/// Operational fees are used for transactions that are essential for the network's operation.
	type OperationalFeeMultiplier = OperationalFeeMultiplier;

	/// Defines how the weight of a transaction is converted to a fee.
	type WeightToFee = WeightToFee;

	/// Defines a constant multiplier for the length (in bytes) of a transaction, applied as an additional fee.
	type LengthToFee = ConstantMultiplier<Balance, ConstU128<{ TRANSACTION_BYTE_FEE }>>;

	/// Defines how the fee multiplier is updated based on the block fullness.
	/// The `TargetedFeeAdjustment` adjusts the fee multiplier to maintain the target block fullness.
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * ANLOG;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

/// ## 08 - <a id="config.Vesting">[`Vesting`] Config</a>
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
