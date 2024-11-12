//! This is our mainnet runtime.
//!
//! ## Governance
//!
//! There are two main governance bodies. The first one is responsible for maintaining the chain
//! and keeping it operational, while the second body is responsible to manage treasury spending and unallocated issuance.
//!
//! ### Technical Committee
//!
//! The technical committee is managed using the [`pallet_collective`], [`pallet_membership`] and our own custom [`pallet_governance`].
//!
//! While the first two pallets tally the votes and manage the membership of the committee, our custom pallet it used to elevate committee origin to more privileged level for selected calls.
//!
#![allow(clippy::identity_op)]
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limits.
#![recursion_limit = "1024"]

use scale_codec::{Decode, Encode, MaxEncodedLen};

use polkadot_sdk::*;

use frame_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
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
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
		ConstantMultiplier, Weight,
	},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureWithSuccess,
};
use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pallet_identity::legacy::IdentityInfo;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_ranked_collective::{MemberIndex, Rank};
use pallet_session::historical as pallet_session_historical;
// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};

use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
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
use sp_std::prelude::*;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

pub use time_primitives::{
	AccountId, Balance, BatchId, BlockHash, BlockNumber, ChainName, ChainNetwork, Commitment,
	ErrorMsg, Gateway, GatewayMessage, MemberStatus, MembersInterface, Moment, NetworkId,
	NetworksInterface, Nonce, PeerId, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, Signature,
	Task, TaskId, TaskResult, TssPublicKey, TssSignature, ANLOG,
};

/// weightToFee implementation
use runtime_common::fee::WeightToFee;
/// Constant values used within the runtime.
use runtime_common::{currency::*, time::*, BABE_GENESIS_EPOCH_CONFIG, MAXIMUM_BLOCK_WEIGHT};
use sp_runtime::generic::Era;

/// Benchmarked pallet weights
mod weights;

/// Generated voter bag information.
mod staking_bags;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Max size for serialized extrinsic params for this testing runtime.
/// This is a quite arbitrary but empirically battle tested value.
#[cfg(test)]
pub const CALL_PARAMS_MAX_SIZE: usize = 244;

/// Default Runtime version.
#[cfg(not(feature = "development"))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-timechain"),
	impl_name: create_runtime_str!("analog-timechain"),
	authoring_version: 0,
	spec_version: 0,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Staging Runtime version.
#[cfg(feature = "development")]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-staging"),
	impl_name: create_runtime_str!("analog-staging"),
	authoring_version: 0,
	spec_version: 0,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
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

pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 2 * HOURS;
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
	const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

	(EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
};

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
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

/// Calls that can bypass the safe-mode pallet.
pub struct SafeModeWhitelistedCalls;
impl Contains<RuntimeCall> for SafeModeWhitelistedCalls {
	fn contains(call: &RuntimeCall) -> bool {
		matches!(call, RuntimeCall::System(_) | RuntimeCall::SafeMode(_))
	}
}

parameter_types! {
	/// This represents duration of the networks safe mode.
	/// Safe mode is typically activated in response to potential threats or instabilities
	/// to restrict certain network operations.
	pub const EnterDuration: BlockNumber = 1 * DAYS;
	/// The deposit amount required to initiate the safe mode entry process.
	/// This ensures that only participants with a significant economic stake (2,000,000 ANLOG tokens)
	/// can trigger safe mode, preventing frivolous or malicious activations.
	pub const EnterDepositAmount: Balance = 5_000_000 * ANLOG;  // ~5.5% of overall supply
	/// The safe mode duration (in blocks) can be to extended.
	/// This represents an additional 2 hours before an extension can be applied,
	/// ensuring that any continuation of safe mode is deliberate and considered.
	pub const ExtendDuration: BlockNumber = 12 * HOURS;
	/// This ensures that participants must provide a moderate economic stake (1,000,000 ANLOG tokens)
	/// to request an extension of safe mode, making it a costly action to prolong the restricted state.
	pub const ExtendDepositAmount: Balance = 2_000_000 * ANLOG;
	/// The minimal duration a deposit will remain reserved after safe-mode is entered or extended,
	/// unless ['Pallet::force_release_deposit'] is successfully called sooner, acts as a security buffer
	/// to ensure stability and allow for safe recovery from critical events
	pub const ReleaseDelay: u32 = 2 * DAYS;
}

impl pallet_safe_mode::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WhitelistedCalls = SafeModeWhitelistedCalls;
	type EnterDuration = EnterDuration;
	type EnterDepositAmount = EnterDepositAmount;
	type ExtendDuration = ExtendDuration;
	type ExtendDepositAmount = ExtendDepositAmount;
	type ForceEnterOrigin = EnsureRootWithSuccess<AccountId, ConstU32<9>>;
	type ForceExtendOrigin = EnsureRootWithSuccess<AccountId, ConstU32<11>>;
	type ForceExitOrigin = EnsureRoot<AccountId>;
	type ForceDepositOrigin = EnsureRoot<AccountId>;
	type ReleaseDelay = ReleaseDelay;
	type Notify = ();
	type WeightInfo = pallet_safe_mode::weights::SubstrateWeight<Runtime>;
}

#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	type BaseCallFilter = SafeMode;
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type DbWeight = RocksDbWeight;
	type Nonce = Nonce;
	type Hash = BlockHash;
	type AccountId = AccountId;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	type SS58Prefix = ConstU16<12850>;
	type MaxConsumers = ConstU32<16>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

parameter_types! {
	/// Base deposit required for storing a multisig execution, covering the cost of a single storage item.
	// One storage item; key size is 32; value is size 4+4(block number)+16(balance)+32(account ID) bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = ConstU32<100>;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	// 32 + proxy_type.encode().len() bytes of data.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const AnnouncementDepositBase: Balance = deposit(1, 16);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 68);
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances(..)
					| RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. })
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::RankedPolls(..)
					| RuntimeCall::RankedCollective(..)
					| RuntimeCall::TechnicalCommittee(..)
					| RuntimeCall::Treasury(..)
			),
			ProxyType::Staking => {
				matches!(c, RuntimeCall::Staking(..))
			},
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	/// An epoch is a unit of time used for key operations in the consensus mechanism, such as validator
	/// rotations and randomness generation. Once set at genesis, this value cannot be changed without
	/// breaking block production.
	///
	/// Note: Do not attempt to modify after chain launch, as it will cause serious issues with block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	/// This defines the interval at which new blocks are produced in the blockchain. It impacts
	/// the speed of transaction processing and finality, as well as the load on the network
	/// and validators. The value is defined in milliseconds.
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
	/// This defines how long a misbehavior reports in the staking or consensus system
	/// remains valid before it expires. It is based on the bonding
	/// duration, sessions per era, and the epoch duration. The longer the bonding duration or
	/// number of sessions per era, the longer reports remain valid.
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
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

parameter_types! {
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

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

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
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
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		TargetBlockFullness,
		AdjustmentVariable,
		MinimumMultiplier,
		MaximumMultiplier,
	>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub grandpa: Grandpa,
		pub babe: Babe,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_020_000,
		max_inflation: 0_080_000,
		ideal_stake: 0_600_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	/// The number of sessions that constitute an era. An era is the time period over which staking rewards
	/// are distributed, and validator set changes can occur. The era duration is a function of the number of
	/// sessions and the length of each session.
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;

	/// The number of eras that a bonded stake must remain locked after the owner has requested to unbond.
	/// This value represents 21 days, assuming each era is 24 hours long. This delay is intended to increase
	/// network security by preventing stakers from immediately withdrawing funds after participating in staking.
	pub const BondingDuration: sp_staking::EraIndex = 24 * 21;

	/// The number of eras after a slashing event before the slashing is enacted. This delay allows participants
	/// to challenge slashes or react to slashing events. It is set to 1/4 of the BondingDuration.
	pub const SlashDeferDuration: sp_staking::EraIndex = 24 * 7;

	/// The reward curve used for staking payouts. This curve defines how rewards are distributed across validators
	/// and nominators, typically favoring higher stakes but ensuring diminishing returns as stakes increase.
	/// The curve is piecewise linear, allowing for different reward distribution models.
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;

	/// The maximum number of validators a nominator can nominate. This sets an upper limit on how many validators
	/// can be supported by a single nominator. A higher number allows more decentralization but increases the
	/// complexity of the staking system.
	pub const MaxNominators: u32 = 64;

	/// The maximum number of controllers that can be included in a deprecation batch when deprecated staking controllers
	/// are being phased out. This helps manage the process of retiring controllers to prevent overwhelming the system
	/// during upgrades.
	pub const MaxControllersInDeprecationBatch: u32 = 5900;

	/// The number of blocks before an off-chain worker repeats a task. Off-chain workers handle tasks that are performed
	/// outside the main blockchain execution, such as fetching data or performing computation-heavy operations. This value
	/// sets how frequently these tasks are repeated.
	pub OffchainRepeat: BlockNumber = 5;

	/// The number of eras that historical staking data is kept in storage. This depth determines how far back the system
	/// keeps records of staking events and rewards for historical queries and audits. Older data beyond this depth will
	/// be pruned to save storage.
	pub HistoryDepth: u32 = 84;
}

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 3, 4>,
	>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxExposurePageSize = ConstU32<256>;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
	// This a placeholder, to be introduced in the next PR as an instance of bags-list
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type HistoryDepth = HistoryDepth;
	type EventListeners = ();
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
}

parameter_types! {
	/// This phase determines the time window, in blocks, during which signed transactions (those that
	/// are authorized by users with private keys, usually nominators or council members) can be submitted.
	/// It is calculated as 1/3 of the total epoch duration, ensuring that signed
	/// transactions are allowed for a quarter of the epoch.
	pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 3;
	/// This phase determines the time window, in blocks, during which unsigned transactions (those
	/// without authorization, usually by offchain workers) can be submitted. Like the signed phase,
	/// it occupies 1/3 of the total epoch duration.
	pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

	// Signed Config
	/// This represents the fixed reward given to participants for submitting valid signed
	/// transactions. It is set to 1 ANLOG token, meaning that any participant who successfully
	/// submits a signed transaction will receive this base reward.
	pub const SignedRewardBase: Balance = 1 * ANLOG;
	/// This deposit ensures that users have economic stakes in the submission of valid signed
	/// transactions. It is currently set to 1 ANLOG, meaning participants must lock 1 ANLOG as
	pub const SignedFixedDeposit: Balance = 1 * ANLOG;
	/// This percentage increase applies to deposits for multiple or repeated signed transactions.
	/// It is set to 10%, meaning that each additional submission after the first will increase the
	/// required deposit by 10%. This serves as a disincentive to spamming the system with repeated
	/// submissions.
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
	/// This deposit ensures that larger signed transactions incur higher costs, reflecting the
	/// increased resource consumption they require. It is set to 10 milliANLOG per byte.
	pub const SignedDepositByte: Balance = deposit(0, 1);

	// Miner Configs
	/// This priority level determines the order in which unsigned transactions are included
	/// in blocks relative to other transactions.
	pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
	/// The maximum weight (computational limit) allowed for miner operations in a block.
	/// This ensures that the block has enough space left for miner operations while maintaining
	/// a limit on overall block execution weight.
	pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
		.saturating_sub(BlockExecutionWeight::get());
	/// This value is set to 90% of the maximum allowed block length for normal transactions.
	/// It ensures that miner solutions do not consume too much block space, leaving enough
	/// room for other transactions.
	pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVotersSolution,
	>(16)
);

parameter_types! {
	// Note: the EPM in this runtime runs the election on-chain. The election bounds must be
	// carefully set so that an election round fits in one block.
	pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(10_000.into()).targets_count(1_500.into()).build();
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(5_000.into()).targets_count(1_250.into()).build();

	pub MaxNominations: u32 = <NposSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
	pub MaxElectingVotersSolution: u32 = 40_000;
	// The maximum winners that can be elected by the Election pallet which is equivalent to the
	// maximum active validators the staking pallet can have.
	pub MaxActiveValidators: u32 = 1000;
}

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.
pub struct ElectionProviderBenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
	fn get() -> Option<BalancingConfig> {
		use sp_runtime::traits::TrailingZeroInput;
		let iterations = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		let config = BalancingConfig { iterations, tolerance: 0 };
		Some(config)
	}
}

pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Bounds = ElectionBoundsOnChain;
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type Solution = NposSolution16;
	type MaxVotesPerVoter =
	<<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxWinners = ConstU32<100>;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MultiPhaseUnsignedPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase =
		GeometricDepositBase<Balance, SignedFixedDeposit, SignedDepositIncreaseFactor>;
	type SignedDepositByte = SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedDepositWeight = ();
	type SignedMaxWeight = MinerMaxWeight;
	type SlashHandler = (); // burn slashes
	type RewardHandler = (); // nothing to do upon rewards
	type DataProvider = Staking;
	type Fallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EnsureRootOrHalfTechnical;
	type MaxWinners = ConstU32<100>;
	type ElectionBounds = ElectionBoundsMultiPhase;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &staking_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	/// The voter bags-list is loosely kept up to date, and the real source of truth for the score
	/// of each node is the staking pallet.
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = VoteWeight;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
}

parameter_types! {
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
	/// TODO: Select good value
	pub const StorageBaseDeposit: Balance = 1 * ANLOG;
	pub const StorageByteDeposit: Balance = deposit(0,1);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<StorageBaseDeposit, StorageByteDeposit, Balance>,
	>;
}

parameter_types! {
	   pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
			   RuntimeBlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxScheduledPerBlock = ConstU32<512>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 100 * ANLOG;
	pub const UndecidingTimeout: BlockNumber = 28 * DAYS;
}

/// ## Referenda
///
/// Most of the governance follows tracks ...
pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
		static DATA: [(u16, pallet_referenda::TrackInfo<Balance, BlockNumber>); 1] = [(
			0u16,
			pallet_referenda::TrackInfo {
				name: "root",
				max_deciding: 1,
				decision_deposit: 10,
				prepare_period: 4,
				decision_period: 4,
				confirm_period: 2,
				min_enactment_period: 4,
				min_approval: pallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(50),
					ceil: Perbill::from_percent(100),
				},
				min_support: pallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(0),
					ceil: Perbill::from_percent(100),
				},
			},
		)];
		&DATA[..]
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
			match system_origin {
				frame_system::RawOrigin::Root => Ok(0),
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}
pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);

impl pallet_referenda::Config for Runtime {
	type WeightInfo = pallet_referenda::weights::SubstrateWeight<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = pallet_balances::Pallet<Self>;
	type SubmitOrigin = EnsureSigned<AccountId>;
	type CancelOrigin = EnsureRoot<AccountId>;
	type KillOrigin = EnsureRoot<AccountId>;
	type Slash = ();
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
}

pub struct MaxMemberCount;
impl MaybeConvert<Rank, MemberIndex> for MaxMemberCount {
	fn maybe_convert(_: Rank) -> Option<MemberIndex> {
		Some(100)
	}
}

impl pallet_ranked_collective::Config for Runtime {
	type WeightInfo = pallet_ranked_collective::weights::SubstrateWeight<Self>;
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = Self::DemoteOrigin;
	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type DemoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type ExchangeOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type Polls = RankedPolls;
	type MinRankOfClass = IdentityT;
	type MaxMemberCount = MaxMemberCount;
	type VoteWeight = pallet_ranked_collective::Geometric;
	type MemberSwappedHandler = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = ();
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance1;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

#[cfg(feature = "development")]
// Limit membership check to development mode
type TechnicalMember = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;

#[allow(dead_code)]
type TechnicalMajority =
	pallet_collective::EnsureProportionMoreThan<AccountId, TechnicalCollective, 1, 2>;
#[allow(dead_code)]
type TechnicalQualifiedMajority =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
#[allow(dead_code)]
type TechnicalSuperMajority =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 3, 4>;
#[allow(dead_code)]
type TechnicalUnanimity =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;

type EnsureRootOrHalfTechnical = EitherOfDiverse<EnsureRoot<AccountId>, TechnicalMajority>;

impl pallet_membership::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrHalfTechnical;
	type RemoveOrigin = EnsureRootOrHalfTechnical;
	type SwapOrigin = EnsureRootOrHalfTechnical;
	type ResetOrigin = EnsureRootOrHalfTechnical;
	type PrimeOrigin = EnsureRootOrHalfTechnical;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * ANLOG;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(1);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * ANLOG;
	pub const DataDepositPerByte: Balance = deposit(0,1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaximumReasonLength: u32 = 300;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = Balance::MAX;
	pub const SpendPayoutPeriod: BlockNumber = 30 * DAYS;
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionMoreThan<AccountId, TechnicalCollective, 1, 2>,
	>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = Bounties;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = SpendPayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 5 * ANLOG;
	pub const BountyDepositBase: Balance = 1 * ANLOG;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 1 * ANLOG;
	pub const CuratorDepositMax: Balance = 100 * ANLOG;
	pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type OnSlash = ();
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
	type ChildBountyManager = ChildBounties;
}

parameter_types! {
	pub const ChildBountyValueMinimum: Balance = 1 * ANLOG;
}

impl pallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = ConstU32<5>;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = pallet_child_bounties::weights::SubstrateWeight<Runtime>;
}

impl pallet_tips::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type OnSlash = ();
	type Tippers = TechnicalMembership;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type MaxTipAmount = ConstU128<{ 500 * ANLOG }>;
	type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
	/// We prioritize im-online heartbeats over election solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;
	pub const MaxAuthorities: u32 = 100;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
		let tip = 0;
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			frame_metadata_hash_extension::CheckMetadataHash::new(false),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = <Runtime as frame_system::Config>::Lookup::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = Babe;
	type ValidatorSet = Historical;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	// difference of 26 bytes on-chain for the registration and 9 bytes on-chain for the identity
	// information, already accounted for by the byte deposit
	pub const BasicDeposit: Balance = deposit(1, 17);
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const SubAccountDeposit: Balance = 2 * ANLOG;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EnsureRootOrHalfTechnical;
	type RegistrarOrigin = EnsureRootOrHalfTechnical;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const MinVestedTransfer: Balance = 100 * ANLOG;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

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

// Custom pallet config
parameter_types! {
	pub IndexerReward: Balance = ANLOG;
}

// Mainnet config

#[cfg(not(feature = "development"))]
/// Default admin origin for system related governance
type SystemAdmin = TechnicalUnanimity;

#[cfg(not(feature = "development"))]
/// Default admin origin for staking related governance
type StakingAdmin = TechnicalSuperMajority;

#[cfg(not(feature = "development"))]
/// Default admin origin for all chronicle related pallets
type ChronicleAdmin = TechnicalQualifiedMajority;

// Staging config

#[cfg(feature = "development")]
/// Development admin origin for all system calls
type SystemAdmin = TechnicalMember;

#[cfg(feature = "development")]
/// Development admin origin for all staking calls
type StakingAdmin = TechnicalMember;

#[cfg(feature = "development")]
/// Development admin origin for all chronicle related pallets
type ChronicleAdmin = TechnicalMember;

impl pallet_members::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::members::WeightInfo<Runtime>;
	type Elections = Elections;
	type Shards = Shards;
	type MinStake = ConstU128<{ 90_000 * ANLOG }>;
	type HeartbeatTimeout = ConstU32<300>;
	type MaxTimeoutsPerBlock = ConstU32<25>;
}

impl pallet_elections::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::elections::WeightInfo<Runtime>;
	type Members = Members;
	type Shards = Shards;
	type Networks = Networks;
	type MaxElectionsPerBlock = ConstU32<5>;
}

impl pallet_shards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::shards::WeightInfo<Runtime>;
	type Members = Members;
	type Elections = Elections;
	type Tasks = Tasks;
	type MaxTimeoutsPerBlock = ConstU32<5>;
	type DkgTimeout = ConstU32<10>;
}

impl pallet_tasks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::tasks::WeightInfo<Runtime>;
	type Networks = Networks;
	type Shards = Shards;
	type MaxTasksPerBlock = ConstU32<50>;
	type MaxBatchesPerBlock = ConstU32<10>;
}

parameter_types! {
 pub const InitialRewardPoolAccount: AccountId = AccountId::new([0_u8; 32]);
 pub const InitialTimegraphAccount: AccountId = AccountId::new([0_u8; 32]);
 pub const InitialThreshold: Balance = 1000;
}

impl pallet_timegraph::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::timegraph::WeightInfo<Runtime>;
	type Currency = Balances;
	type InitialRewardPoolAccount = InitialRewardPoolAccount;
	type InitialTimegraphAccount = InitialTimegraphAccount;
	type InitialThreshold = InitialThreshold;
}

impl pallet_networks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::networks::WeightInfo<Runtime>;
	type Tasks = Tasks;
}

impl pallet_dmail::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::dmail::WeightInfo<Runtime>;
}

impl pallet_governance::Config for Runtime {
	type SystemAdmin = SystemAdmin;
	type StakingAdmin = StakingAdmin;
}

/// Main runtime assembly
#[frame_support::runtime]
mod runtime {
	use super::*;

	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	// = SDK pallets =

	// Core pallets
	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	// Block production, finality, heartbeat and discovery
	#[runtime::pallet_index(2)]
	pub type Babe = pallet_babe;

	#[runtime::pallet_index(3)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(4)]
	pub type ImOnline = pallet_im_online;

	#[runtime::pallet_index(5)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	// Tokens, rewards, fees and vesting
	#[runtime::pallet_index(6)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(7)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(8)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(9)]
	pub type Vesting = pallet_vesting;

	// Batch, proxy and multisig support
	#[runtime::pallet_index(10)]
	pub type Utility = pallet_utility;

	#[runtime::pallet_index(11)]
	pub type Proxy = pallet_proxy;

	#[runtime::pallet_index(12)]
	pub type Multisig = pallet_multisig;

	// Nominated proof of stake
	#[runtime::pallet_index(13)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase;

	#[runtime::pallet_index(14)]
	pub type VoterList = pallet_bags_list<Instance1>;

	#[runtime::pallet_index(15)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(16)]
	pub type Offences = pallet_offences;

	#[runtime::pallet_index(17)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(18)]
	pub type Historical = pallet_session_historical;

	// On-chain storage, scheduler and identity
	#[runtime::pallet_index(19)]
	pub type Preimage = pallet_preimage;

	#[runtime::pallet_index(20)]
	pub type Scheduler = pallet_scheduler;

	#[runtime::pallet_index(21)]
	pub type Identity = pallet_identity;

	// On-chain governance
	#[runtime::pallet_index(22)]
	pub type TechnicalCommittee = pallet_collective<Instance1>;

	#[runtime::pallet_index(23)]
	pub type TechnicalMembership = pallet_membership;

	#[runtime::pallet_index(24)]
	pub type RankedCollective = pallet_ranked_collective;

	#[runtime::pallet_index(25)]
	pub type RankedPolls = pallet_referenda;

	// 27 is reserved for sudo

	// On-chain funding
	#[runtime::pallet_index(28)]
	pub type Treasury = pallet_treasury;

	#[runtime::pallet_index(29)]
	pub type Bounties = pallet_bounties;

	#[runtime::pallet_index(30)]
	pub type ChildBounties = pallet_child_bounties;

	#[runtime::pallet_index(31)]
	pub type Tips = pallet_tips;

	// = Custom pallets =

	// general message passing pallets
	#[runtime::pallet_index(32)]
	pub type Members = pallet_members;

	#[runtime::pallet_index(33)]
	pub type Shards = pallet_shards;

	#[runtime::pallet_index(34)]
	pub type Elections = pallet_elections;

	#[runtime::pallet_index(35)]
	pub type Tasks = pallet_tasks;

	#[runtime::pallet_index(36)]
	pub type Timegraph = pallet_timegraph;

	#[runtime::pallet_index(37)]
	pub type Networks = pallet_networks;

	// Custom governance
	#[runtime::pallet_index(38)]
	pub type Governance = pallet_governance;

	#[runtime::pallet_index(39)]
	pub type Dmail = pallet_dmail;

	// = Temp pallets =

	// Pallet to control the initial launch
	#[runtime::pallet_index(42)]
	pub type SafeMode = pallet_safe_mode;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = runtime_common::SignedExtra<Runtime>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

// All migrations executed on runtime upgrade implementing `OnRuntimeUpgrade`.
type Migrations = ();

/// List of available benchmarks
#[cfg(feature = "runtime-benchmarks")]
mod benches {
	polkadot_sdk::frame_benchmarking::define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_babe, Babe]
		[pallet_bags_list, VoterList]
		[pallet_balances, Balances]
		[pallet_bounties, Bounties]
		[pallet_child_bounties, ChildBounties]
		[pallet_collective, TechnicalCommittee]
		[pallet_elections, Elections]
		[pallet_dmail, Dmail]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pallet_election_provider_support_benchmarking, EPSBench::<Runtime>]
		[pallet_grandpa, Grandpa]
		[pallet_identity, Identity]
		[pallet_im_online, ImOnline]
		[pallet_membership, TechnicalMembership]
		[pallet_members, Members]
		[pallet_multisig, Multisig]
		[pallet_networks, Networks]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_preimage, Preimage]
		[pallet_proxy, Proxy]
		[pallet_ranked_collective, RankedCollective]
		[pallet_referenda, RankedPolls]
		[pallet_scheduler, Scheduler]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_shards, Shards]
		[pallet_staking, Staking]
		[pallet_tasks, Tasks]
		[pallet_timegraph, Timegraph]
		[pallet_timestamp, Timestamp]
		[pallet_tips, Tips]
		[pallet_treasury, Treasury]
		[pallet_utility, Utility]
		[pallet_vesting, Vesting]
		[pallet_safe_mode, SafeMode]
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			use scale_codec::Encode;

			Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}

		fn eras_stakers_page_count(era: sp_staking::EraIndex, account: AccountId) -> sp_staking::Page {
			Staking::api_eras_stakers_page_count(era, account)
		}

		fn pending_rewards(era: sp_staking::EraIndex, account: AccountId) -> bool {
			Staking::api_pending_rewards(era, account)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(BABE_GENESIS_EPOCH_CONFIG);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use scale_codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<AuthorityDiscoveryId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(call: RuntimeCall, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(call: RuntimeCall, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
			   fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
					   SessionKeys::generate(seed)
			   }

			   fn decode_session_keys(
					   encoded: Vec<u8>,
			   ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
					   SessionKeys::decode_into_raw_public_keys(&encoded)
			   }
	   }

	impl time_primitives::MembersApi<Block> for Runtime {
		fn get_member_peer_id(account: &AccountId) -> Option<PeerId> {
			Members::member_peer_id(account)
		}

		fn get_heartbeat_timeout() -> BlockNumber {
			Members::get_heartbeat_timeout()
		}

		fn get_min_stake() -> Balance {
			Members::get_min_stake()
		}
	}

	impl time_primitives::NetworksApi<Block> for Runtime {
		fn get_network(network_id: NetworkId) -> Option<(ChainName, ChainNetwork)> {
			Networks::get_network(network_id)
		}

		fn get_gateway(network: NetworkId) -> Option<Gateway> {
			Networks::gateway(network)
		}
	}

	impl time_primitives::ShardsApi<Block> for Runtime {
		fn get_shards(account: &AccountId) -> Vec<ShardId> {
			Shards::get_shards(account)
		}

		fn get_shard_members(shard_id: ShardId) -> Vec<(AccountId, MemberStatus)> {
			Shards::get_shard_members(shard_id)
		}

		fn get_shard_threshold(shard_id: ShardId) -> u16 {
			Shards::get_shard_threshold(shard_id)
		}

		fn get_shard_status(shard_id: ShardId) -> ShardStatus {
			Shards::get_shard_status(shard_id)
		}

		fn get_shard_commitment(shard_id: ShardId) -> Option<Commitment> {
			Shards::get_shard_commitment(shard_id)
		}
	}

	impl time_primitives::TasksApi<Block> for Runtime {
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskId> {
			Tasks::get_shard_tasks(shard_id)
		}

		fn get_task(task_id: TaskId) -> Option<Task>{
			Tasks::get_task(task_id)
		}

		fn get_task_submitter(task_id: TaskId) -> Option<PublicKey> {
			Tasks::get_task_submitter(task_id)
		}

		fn get_task_result(task_id: TaskId) -> Option<Result<(), ErrorMsg>>{
			Tasks::get_task_result(task_id)
		}

		fn get_task_shard(task_id: TaskId) -> Option<ShardId>{
			Tasks::get_task_shard(task_id)
		}

		fn get_batch_message(batch_id: BatchId) -> Option<GatewayMessage> {
			Tasks::get_batch_message(batch_id)
		}
	}

	impl time_primitives::SubmitTransactionApi<Block> for Runtime {
		fn submit_transaction(encoded_transaction: Vec<u8>) -> Result<(), ()> {
			sp_io::offchain::submit_transaction(encoded_transaction)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let mut whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			// Treasury Account
			// TODO: this is manual for now, someday we might be able to use a
			// macro for this particular key
			let treasury_key = frame_system::Account::<Runtime>::hashed_key_for(Treasury::account_id());
			whitelist.push(treasury_key.to_vec().into());

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);
			Ok(batches)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_election_provider_support::NposSolution;
	use frame_system::offchain::CreateSignedTransaction;
	use pallet_elections::WeightInfo as W4;
	use pallet_members::WeightInfo as W2;
	use pallet_shards::WeightInfo as W3;
	use pallet_tasks::WeightInfo;
	use sp_runtime::UpperOf;
	use time_primitives::ON_INITIALIZE_BOUNDS;

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>()
		where
			T: CreateSignedTransaction<RuntimeCall>,
		{
		}

		is_submit_signed_transaction::<Runtime>();
	}

	#[test]
	fn perbill_as_onchain_accuracy() {
		type OnChainAccuracy =
			<<Runtime as pallet_election_provider_multi_phase::MinerConfig>::Solution as NposSolution>::Accuracy;
		let maximum_chain_accuracy: Vec<UpperOf<OnChainAccuracy>> = (0..MaxNominations::get())
			.map(|_| <UpperOf<OnChainAccuracy>>::from(OnChainAccuracy::one().deconstruct()))
			.collect();
		let _: UpperOf<OnChainAccuracy> =
			maximum_chain_accuracy.iter().fold(0, |acc, x| acc.checked_add(*x).unwrap());
	}

	#[test]
	fn call_size() {
		let size = core::mem::size_of::<RuntimeCall>();
		assert!(
			size <= CALL_PARAMS_MAX_SIZE,
			"size of RuntimeCall {} is more than {CALL_PARAMS_MAX_SIZE} bytes.
			 Some calls have too big arguments, use Box to reduce the size of RuntimeCall.
			 If the limit is too strong, maybe consider increase the limit.",
			size,
		);
	}

	#[test]
	fn max_tasks_per_block() {
		let avg_on_initialize: Weight =
			ON_INITIALIZE_BOUNDS.tasks * (AVERAGE_ON_INITIALIZE_RATIO * MAXIMUM_BLOCK_WEIGHT);
		assert!(
			<Runtime as pallet_tasks::Config>::WeightInfo::schedule_tasks(1)
				.all_lte(avg_on_initialize),
			"BUG: Scheduling a single task consumes more weight than available in on-initialize"
		);
		assert!(
			<Runtime as pallet_tasks::Config>::WeightInfo::schedule_tasks(1)
				.all_lte(<Runtime as pallet_tasks::Config>::WeightInfo::schedule_tasks(2)),
			"BUG: Scheduling 1 task consumes more weight than scheduling 2"
		);
		let mut num_tasks: u32 = 2;
		while <Runtime as pallet_tasks::Config>::WeightInfo::schedule_tasks(num_tasks)
			.all_lt(avg_on_initialize)
		{
			num_tasks += 1;
			if num_tasks == 10_000_000 {
				// 10_000_000 tasks reached; halting to break out of loop
				break;
			}
		}
		let max_tasks_per_block_configured: u32 =
			<Runtime as pallet_tasks::Config>::MaxTasksPerBlock::get();
		assert!(
			max_tasks_per_block_configured <= num_tasks,
			"MaxTasksPerBlock {max_tasks_per_block_configured} > max number of tasks per block tested = {num_tasks}"
		);
	}

	#[test]
	fn max_batches_per_block() {
		let avg_on_initialize: Weight =
			ON_INITIALIZE_BOUNDS.batches * (AVERAGE_ON_INITIALIZE_RATIO * MAXIMUM_BLOCK_WEIGHT);
		assert!(
			<Runtime as pallet_tasks::Config>::WeightInfo::prepare_batches(1)
				.all_lte(avg_on_initialize),
			"BUG: Starting a single batch consumes more weight than available in on-initialize"
		);
		assert!(
			<Runtime as pallet_tasks::Config>::WeightInfo::prepare_batches(1)
				.all_lte(<Runtime as pallet_tasks::Config>::WeightInfo::prepare_batches(2)),
			"BUG: Starting 1 batch consumes more weight than starting 2"
		);
		let mut num_batches: u32 = 2;
		while <Runtime as pallet_tasks::Config>::WeightInfo::prepare_batches(num_batches)
			.all_lt(avg_on_initialize)
		{
			num_batches += 1;
			if num_batches == 10_000_000 {
				// 10_000_000 batches started; halting to break out of loop
				break;
			}
		}
		let max_batches_per_block_configured: u32 =
			<Runtime as pallet_tasks::Config>::MaxBatchesPerBlock::get();
		assert!(
			max_batches_per_block_configured <= num_batches,
			"MaxBatchesPerBlock {max_batches_per_block_configured} > max number of batches per block tested = {num_batches}"
		);
	}

	#[test]
	fn max_dkg_timeouts_per_block() {
		let avg_on_initialize: Weight =
			ON_INITIALIZE_BOUNDS.dkgs * (AVERAGE_ON_INITIALIZE_RATIO * MAXIMUM_BLOCK_WEIGHT);
		assert!(
			<Runtime as pallet_shards::Config>::WeightInfo::timeout_dkgs(1)
				.all_lte(avg_on_initialize),
			"BUG: One DKG timeout consumes more weight than available in on-initialize"
		);
		assert!(
			<Runtime as pallet_shards::Config>::WeightInfo::timeout_dkgs(1)
				.all_lte(<Runtime as pallet_shards::Config>::WeightInfo::timeout_dkgs(2)),
			"BUG: 1 DKG timeout consumes more weight than 2 DKG timeouts"
		);
		let mut num_timeouts: u32 = 2;
		while <Runtime as pallet_shards::Config>::WeightInfo::timeout_dkgs(num_timeouts)
			.all_lt(avg_on_initialize)
		{
			num_timeouts += 1;
			if num_timeouts == 10_000_000 {
				// 10_000_000 timeouts; halting to break out of loop
				break;
			}
		}
		let max_timeouts_per_block: u32 =
			<Runtime as pallet_shards::Config>::MaxTimeoutsPerBlock::get();
		assert!(
			max_timeouts_per_block <= num_timeouts,
			"MaxDKGTimeoutsPerBlock {max_timeouts_per_block} > max number of DKG timeouts per block tested = {num_timeouts}"
		);
	}

	#[test]
	fn max_heartbeat_timeouts_per_block() {
		let avg_on_initialize: Weight =
			ON_INITIALIZE_BOUNDS.heartbeats * (AVERAGE_ON_INITIALIZE_RATIO * MAXIMUM_BLOCK_WEIGHT);
		assert!(
			<Runtime as pallet_members::Config>::WeightInfo::timeout_heartbeats(1)
				.all_lte(avg_on_initialize),
			"BUG: One Heartbeat timeout consumes more weight than available in on-initialize"
		);
		assert!(
			<Runtime as pallet_members::Config>::WeightInfo::timeout_heartbeats(1)
				.all_lte(<Runtime as pallet_members::Config>::WeightInfo::timeout_heartbeats(2)),
			"BUG: 1 Heartbeat timeout consumes more weight than 2 Heartbeat timeouts"
		);
		let mut num_timeouts: u32 = 2;
		while <Runtime as pallet_members::Config>::WeightInfo::timeout_heartbeats(num_timeouts)
			.all_lt(avg_on_initialize)
		{
			num_timeouts += 1;
			if num_timeouts == 10_000_000 {
				// 10_000_000 timeouts; halting to break out of loop
				break;
			}
		}
		let max_timeouts_per_block: u32 =
			<Runtime as pallet_members::Config>::MaxTimeoutsPerBlock::get();
		assert!(
			max_timeouts_per_block <= num_timeouts,
			"MaxHeartbeatTimeoutsPerBlock {max_timeouts_per_block} > max number of Heartbeat timeouts per block tested = {num_timeouts}"
		);
	}

	#[test]
	fn max_elections_per_block() {
		let avg_on_initialize: Weight =
			ON_INITIALIZE_BOUNDS.elections * (AVERAGE_ON_INITIALIZE_RATIO * MAXIMUM_BLOCK_WEIGHT);
		const NUM_UNASSIGNED: u32 = 10;
		let try_elect_shard: Weight =
			<Runtime as pallet_elections::Config>::WeightInfo::try_elect_shard(NUM_UNASSIGNED);
		assert!(
			try_elect_shard.all_lte(avg_on_initialize),
			"BUG: One shard election consumes more weight than available in on-initialize"
		);
		let mut try_elect_shards = try_elect_shard.saturating_add(try_elect_shard);
		assert!(
			try_elect_shard.all_lte(try_elect_shards),
			"BUG: 1 shard election consumes more weight than 2 shard elections"
		);
		let mut num_elections: u32 = 2;
		while try_elect_shards.all_lt(avg_on_initialize) {
			try_elect_shards = try_elect_shards.saturating_add(try_elect_shard);
			num_elections += 1;
			if num_elections == 10_000_000 {
				// 10_000_000 elections; halting to break out of loop
				break;
			}
		}
		let max_elections_per_block: u32 =
			<Runtime as pallet_elections::Config>::MaxElectionsPerBlock::get();
		assert!(
			max_elections_per_block <= num_elections,
			"MaxElectionsPerBlock {max_elections_per_block} > max number of Elections per block tested = {num_elections}"
		);
	}
}
