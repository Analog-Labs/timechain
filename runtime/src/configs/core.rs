//! Frame base configuration

//use scale_codec::Encode;
use static_assertions::const_assert;

use polkadot_sdk::*;

use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	parameter_types,
	traits::{ConstU16, ConstU32},
	weights::{constants::ParityDbWeight, Weight},
};
use frame_system::limits::{BlockLength, BlockWeights};
use sp_version::RuntimeVersion;

use sp_runtime::{
	//generic::Era,
	//traits::{Extrinsic, OpaqueKeys, SaturatedConversion, StaticLookup, Verify},
	//traits::OpaqueKeys,
	Perbill,
};
use time_primitives::{BlockHash, BlockNumber, Moment, SS58_PREFIX}; //Signature

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};

// Local module imports
#[cfg(not(feature = "testnet"))]
use crate::SafeMode;
use crate::{
	weights::{self, BlockExecutionWeight, ExtrinsicBaseWeight},
	AccountId,
	Babe,
	Balance,
	Block,
	Nonce,
	PalletInfo,
	Runtime,
	RuntimeCall,
	RuntimeEvent,
	RuntimeOrigin,
	RuntimeTask,
	//SignedPayload, System, UncheckedExtrinsic,
	MAXIMUM_BLOCK_WEIGHT,
	SLOT_DURATION,
	VERSION,
};

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 4096;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

/// ## 00 - <a id="config.System">[`System`] Config</a>
///
/// Represents the runtime base configuration, pretty standard appart from:
/// - [`BaseCallFilter`](#associatedtype.BaseCallFilter) is currently set to support safe mode
/// - [`AccountData`](#associatedtype.AccountData) is managed by [`Balances`]
/// - [`SS58Prefix`](#associatedtype.SS58Prefix) is registered for mainnet, rest is unofficial
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	#[cfg(not(feature = "testnet"))]
	type BaseCallFilter = SafeMode;
	#[cfg(feature = "testnet")]
	type BaseCallFilter = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type DbWeight = ParityDbWeight;
	type Nonce = Nonce;
	type Hash = BlockHash;
	type AccountId = AccountId;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type SS58Prefix = ConstU16<{ SS58_PREFIX }>;
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	/// minimum valid timeperiod
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

/// ## 01 - <a id="config.Timestamp">[`Timestamp`] Config</a>
///
/// timestamp extension
impl pallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}
