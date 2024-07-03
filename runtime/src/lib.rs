//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "1024"]

// Automatically generated nomination bagging
mod bag_thresholds;

#[cfg(test)]
mod tests;
mod weights;

// Make the WASM binary available in native code
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use polkadot_sdk::*;

use scale_codec::{Decode, Encode};

use frame_election_provider_support::bounds::{ElectionBounds, ElectionBoundsBuilder};
use frame_election_provider_support::{
	generate_solution_type, onchain, ExtendedBalance, SequentialPhragmen,
};
use frame_support::{
	dispatch::DispatchClass,
	traits::{
		tokens::{PayFromAccount, UnityAssetBalanceConversion},
		EitherOfDiverse, Imbalance,
	},
	weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use frame_system::EnsureRootWithSuccess;
use frame_system::{limits::BlockWeights, EnsureRoot};

use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};

use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str,
	generic::{self, Era},
	impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, BlockNumberProvider, Hash as HashT, IdentityLookup,
		NumberFor, OpaqueKeys, Saturating,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, FixedPointNumber, Percent, SaturatedConversion,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use runtime_common::{
	currency::*,
	prod_or_dev,
	weights::{BlockExecutionWeight, ExtrinsicBaseWeight},
};
pub use time_primitives::{
	AccountId, Balance, BlockNumber, ChainName, ChainNetwork, Commitment, DepreciationRate,
	MemberStatus, MemberStorage, NetworkId, PeerId, ProofOfKnowledge, PublicKey, ShardId,
	ShardStatus, Signature, TaskDescriptor, TaskExecution, TaskId, TaskPhase, TaskResult,
	TssPublicKey, TssSignature,
};

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	derive_impl,
	genesis_builder_helper::{build_state, get_preset},
	pallet_prelude::Get,
	parameter_types,
	traits::{
		ConstU128, ConstU16, ConstU32, ConstU64, ConstU8, Currency, EnsureOrigin,
		KeyOwnerProofSystem, OnUnbalanced, Randomness, StorageInfo,
	},
	weights::{constants::RocksDbWeight, ConstantMultiplier, IdentityFee, Weight, WeightToFee},
	PalletId, StorageValue,
};
#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use pallet_timestamp::Call as TimestampCall;
#[cfg(any(feature = "std", test))]
pub use pallet_utility::Call as UtilityCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

pub use sp_runtime::{traits::Bounded, Perbill, Permill, Perquintill};
use static_assertions::const_assert;

pub type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
use pallet_staking::UseValidatorsMap;
pub struct StakingBenchmarkingConfig;

#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;

pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: (1, 4),
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// We assume that an on-initialize consumes 1% of the weight on average, hence a single extrinsic
/// will not be allowed to consume more than `AvailableBlockRatio - 1%`.
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(1);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = time_primitives::BlockHash;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
		pub grandpa: Grandpa,
		pub im_online: ImOnline,
		pub authority_discovery: AuthorityDiscovery,
	}
}

// To learn more about runtime versioning, see:
// <https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning>
#[cfg(not(feature = "development"))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-testnet"),
	impl_name: create_runtime_str!("analog-testnet"),
	authoring_version: 1,
	spec_version: 120,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	state_version: 1,
};

#[cfg(feature = "development")]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-development"),
	impl_name: create_runtime_str!("analog-development"),
	authoring_version: 1,
	spec_version: 120,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	state_version: 1,
};

#[macro_export]
macro_rules! impl_elections_weights {
	($runtime:ident) => {
		parameter_types! {
			/// A limit for off-chain phragmen unsigned solution submission.
			///
			/// We want to keep it as high as possible, but can't risk having it reject,
			/// so we always subtract the base block execution weight.
			pub OffchainSolutionWeightLimit: Weight = BlockWeights::get()
				.get(DispatchClass::Normal)
				.max_extrinsic
				.expect("Normal extrinsics have weight limit configured by default; qed")
				.saturating_sub($runtime::weights::BlockExecutionWeight::get());

			/// A limit for off-chain phragmen unsigned solution length.
			///
			/// We allow up to 90% of the block's size to be consumed by the solution.
			pub OffchainSolutionLengthLimit: u32 = Perbill::from_rational(90_u32, 100) *
				*BlockLength::get()
				.max
				.get(DispatchClass::Normal);
		}
	};
}

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const EPOCH_DURATION_IN_SLOTS: BlockNumber = prod_or_dev!(8 * HOURS, 5 * MINUTES);

const MILLISECONDS_PER_YEAR: u64 = 1000 * 3600 * 24 * 36525 / 100;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}
parameter_types! {
	// The maximum number of nominating and validating accounts
	pub const MaxElectingNominators: u32 = 100;
	pub const MaxElectableValidators: u16 = 5000;

	// The maximum winners that can be elected by the Election pallet which is equivalent to the
	// maximum active validators the staking pallet can have.
	pub MaxActiveValidators: u32 = 1000;
}

impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxValidators = MaxActiveValidators;
	type MaxNominators = MaxNominations;
}

parameter_types! {
	pub const MaxAuthorities: u32 = 100_000;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
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

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
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
	pub CollectivesMaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
	pub const SS58Prefix: u16 = 12850;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`SoloChainDefaultConfig`](`struct@frame_system::config_preludes::SolochainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The index type for blocks.
	type Block = Block;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = weights::system::WeightInfo<Runtime>;
	/// This is used as an identifier of the chain.
	type SS58Prefix = SS58Prefix;
	/// The maximum number of consumers allowed on a single account.
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

parameter_types! {
	pub EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS as u64;

	pub const ExpectedBlockTime: u64 = MILLISECS_PER_BLOCK;
	pub ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type DisabledValidators = Session;
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
	type WeightInfo = (); //weights::babe::WeightInfo<Runtime>;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominations;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type WeightInfo = (); //weights::grandpa::WeightInfo<Runtime>;
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominations;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub NposSolutionPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::max_value();
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = Historical;
	type NextSessionRotation = Babe;
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	// phase durations. 1/8 (1h) of the last session for each.
	pub SignedPhase: u32 = prod_or_dev!(EPOCH_DURATION_IN_SLOTS / 8, 5 * MINUTES);
	pub UnsignedPhase: u32 = prod_or_dev!(EPOCH_DURATION_IN_SLOTS / 8, 5 * MINUTES);

	// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	pub const SignedMaxRefunds: u32 = 16 / 4;
	// TODO fixed deposit..
	pub const SignedDepositBase: Balance = 10;
	// 0.01 DOT per KB of solution data.
	pub const SignedDepositByte: Balance = 10;
	// TODO Each good submission will get 1 DOT as reward
	pub SignedRewardBase: Balance = 1000;

	// 4 hour session, 1 hour unsigned phase, 32 offchain executions.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / prod_or_dev!(32, 10);
}

generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingNominators,
	>(16)
);

parameter_types! {
	// TODO Needs to be properly configured.
	pub const SessionsPerEra: sp_staking::SessionIndex = prod_or_dev!(1, 3);
	pub const BondingDuration: sp_staking::EraIndex = prod_or_dev!(2, 4);
	pub const SlashDeferDuration: sp_staking::EraIndex = 0;//24 * 7; // 1/4 the bonding duration.

	pub const MaxNominations: u32 = <NposCompactSolution16 as frame_election_provider_support::NposSolution>::LIMIT as u32;
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

/// A source of random balance for NposSolver, which is meant to be run by the OCW election miner.
pub struct OffchainRandomBalancing;
impl Get<Option<(usize, ExtendedBalance)>> for OffchainRandomBalancing {
	fn get() -> Option<(usize, ExtendedBalance)> {
		use sp_runtime::traits::TrailingZeroInput;
		let iters = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed")
					% max.saturating_add(1);
				random as usize
			},
		};

		Some((iters, 0))
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

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = prod_or_dev!(5 * DAYS, HOURS);
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = CollectivesMaxProposalWeight;
}

parameter_types! {
	/// Targeted staking rate
	pub const IdealStakingRate: Perquintill = Perquintill::from_percent(75);
	/// Minimum inflation rate
	pub const MinAnnualInflation: Perquintill = Perquintill::from_percent(2);
	/// Maximum inflation rate
	pub const MaxAnnualInflation: Perquintill = Perquintill::from_percent(8);
	/// Reward curve falloff
	pub const RewardFalloff: Perquintill = Perquintill::from_percent(5);
}

/// Custom era payouts that adjust staking rate based on total chronicle stake
pub struct EraPayout;
impl pallet_staking::EraPayout<Balance> for EraPayout {
	fn era_payout(
		total_staked: Balance,
		total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		let total_staked = total_staked + Members::total_stake();
		let stake = Perquintill::from_rational(total_staked, total_issuance);

		let adjustment = pallet_staking_reward_fn::compute_inflation(
			stake,
			IdealStakingRate::get(),
			RewardFalloff::get(),
		);
		let delta_annual_inflation =
			MaxAnnualInflation::get().saturating_sub(MinAnnualInflation::get());

		let staking_inflation =
			MinAnnualInflation::get().saturating_add(delta_annual_inflation * adjustment);
		let period_fraction =
			Perquintill::from_rational(era_duration_millis, MILLISECONDS_PER_YEAR);

		let max_payout = period_fraction * MaxAnnualInflation::get() * total_issuance;
		let staking_payout = (period_fraction * staking_inflation) * total_issuance;
		let rest = max_payout.saturating_sub(staking_payout);

		(staking_payout, rest)
	}
}

parameter_types! {
	pub const MaxExposurePageSize: u32 = 256;
	pub const MaxControllersInDeprecationBatch: u32 = 512;
}

impl pallet_staking::Config for Runtime {
	/// A super-majority of the council can cancel the slash.
	type AdminOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
	>;
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVote;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type SessionInterface = Self;
	type EraPayout = EraPayout;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type MaxUnlockingChunks = ConstU32<32>;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type HistoryDepth = frame_support::traits::ConstU32<84>;
	type TargetList = UseValidatorsMap<Self>;
	type EventListeners = ();
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type MaxExposurePageSize = MaxExposurePageSize;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
}

parameter_types! {
	pub const MinerMaxLength: u32 = 256;
	pub MinerMaxWeight: Weight = RuntimeBlockWeights::get().max_block;

	pub const SignedFixedDeposit: Balance = deposit(2, 0);
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);

	// Note: the EPM in this runtime runs the election on-chain. The election bounds must be
	// carefully set so that an election round fits in one block.
	pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(MaxElectingNominators::get().into()).targets_count((MaxElectableValidators::get() as u32).into()).build();
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(MaxElectingNominators::get().into()).targets_count((MaxElectableValidators::get() as u32).into()).build();
}

impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type Solution = NposCompactSolution16;
	type MaxVotesPerVoter = <
		<Self as pallet_election_provider_multi_phase::Config>::DataProvider
		as
		frame_election_provider_support::ElectionDataProvider
	>::MaxVotesPerVoter;
	type MaxWinners = MaxActiveValidators;

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

/// The numbers configured here could always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory.
pub struct BenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for BenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const ACTIVE_VOTERS: [u32; 2] = [200, 400];
	const TARGETS: [u32; 2] = [1000, 5000];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = MaxElectingNominators::get();
	const MINER_MAXIMUM_VOTERS: u32 = MaxElectingNominators::get();
	const MAXIMUM_TARGETS: u32 = MaxElectableValidators::get() as u32;
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterSignedThreshold = ();
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = NposSolutionPriority;
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
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, ()>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type MaxWinners = MaxActiveValidators;
	type BenchmarkingConfig = BenchmarkConfig;
	type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Self>;
	type ElectionBounds = ElectionBoundsMultiPhase;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = weights::timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const BagThresholds: &'static [u64] = &bag_thresholds::THRESHOLDS;
}

impl pallet_bags_list::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ScoreProvider = Staking;
	type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
	type BagThresholds = BagThresholds;
	type Score = sp_npos_elections::VoteWeight;
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = weights::balances::WeightInfo<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
}

pub type NegativeImbalance<T> = <pallet_balances::Pallet<T> as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type EventHandler = (Staking, ImOnline);
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for ToAuthor<R>
where
	R: pallet_balances::Config + pallet_authorship::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_nonzero_unbalanced(amount: NegativeImbalance<R>) {
		if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
			<pallet_balances::Pallet<R>>::resolve_creating(&author, amount);
		}
	}
}

pub struct DealWithFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<NegativeImbalance<R>> for DealWithFees<R>
where
	R: pallet_balances::Config + pallet_treasury::Config + pallet_authorship::Config,
	pallet_treasury::Pallet<R>: OnUnbalanced<NegativeImbalance<R>>,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance<R>>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 20% to treasury, 80% to author
			let split = fees.ration(80, 20);
			use pallet_treasury::Pallet as Treasury;
			<Treasury<R> as OnUnbalanced<_>>::on_unbalanced(split.1);
			<ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(split.0);
		}
	}
}

parameter_types! {
	/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
	/// than this will decrease the weight and more will increase.
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
	/// change the fees more rapidly.
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(75, 1_000_000);
	/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
	/// that combined with `AdjustmentVariable`, we can recover from the minimum.
	/// See `multiplier_can_grow_from_zero`.
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
	/// The maximum amount of the multiplier.
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
impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees<Self>>;
	// multiplier to boost the priority of operational transactions
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = ConstantMultiplier<Balance, ConstU128<{ TRANSACTION_BYTE_FEE }>>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const MinReportsPerCommittedOffense: u8 = 1;
	pub const MaxChronicleWorkers: u32 = 5;
	pub const MaxTimeouts: u8 = 2;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: PublicKey,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(
		RuntimeCall,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
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
		log::info!(
			"create_transaction from account {:?} with nonce {} on block {}",
			account,
			nonce,
			current_block
		);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = account;
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (sp_runtime::MultiAddress::Id(address), signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = PublicKey;
	type Signature = Signature;
}

parameter_types! {
	pub MinVestedTransfer: Balance = ANLOG;
	pub const MaxVestingSchedules: u32 = 100;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaxApprovals: u32 = 100;
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = ANLOG;
	pub const SpendPeriod: BlockNumber = prod_or_dev!(DAYS, HOURS);
	pub const Burn: Permill = Permill::from_percent(50);
	pub const MaxBalance: Balance = Balance::max_value();
	pub const PayoutPeriod: BlockNumber = prod_or_dev!(14 * DAYS, 6 * HOURS);
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub struct SubstrateBlockNumberProvider;
impl BlockNumberProvider for SubstrateBlockNumberProvider {
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		System::block_number()
	}
}

impl pallet_treasury::Config for Runtime {
	type Currency = Balances;
	type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type PalletId = TreasuryPalletId;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
	type MaxApprovals = MaxApprovals;
	type SpendOrigin = EnsureRootWithSuccess<AccountId, MaxBalance>;
	type AssetKind = ();
	type Beneficiary = Self::AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = PayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

parameter_types! {
	pub IndexerReward: Balance = ANLOG;
}

impl pallet_members::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::members::WeightInfo<Runtime>;
	type Elections = Elections;
	type MinStake = ConstU128<{ 90_000 * ANLOG }>;
	type HeartbeatTimeout = ConstU32<300>;
}

impl pallet_elections::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::elections::WeightInfo<Runtime>;
	type Members = Members;
	type Shards = Shards;
}

impl pallet_shards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::shards::WeightInfo<Runtime>;
	type Members = Members;
	type Elections = Elections;
	type TaskScheduler = Tasks;
	type DkgTimeout = ConstU32<10>;
}

parameter_types! {
	// PalletId used for generating task accounts to fund read tasks.
	pub const TaskPalletId: PalletId = PalletId(*b"py/tasks");
	// Rewards decline by 1% every 20 blocks.
	pub const RewardDeclineRate: DepreciationRate<BlockNumber> = DepreciationRate { blocks: 20, percent: Percent::from_percent(1) };
}

impl pallet_tasks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::tasks::WeightInfo<Runtime>;
	type Elections = Elections;
	type Shards = Shards;
	type Members = Members;
	type BaseReadReward = ConstU128<{ 2 * ANLOG }>;
	type BaseWriteReward = ConstU128<{ 2 * ANLOG }>;
	type BaseSendMessageReward = ConstU128<{ 2 * ANLOG }>;
	type RewardDeclineRate = RewardDeclineRate;
	type SignPhaseTimeout = ConstU32<10>;
	type WritePhaseTimeout = ConstU32<10>;
	type ReadPhaseTimeout = ConstU32<10>;
	type PalletId = TaskPalletId;
}

impl pallet_timegraph::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::timegraph::WeightInfo<Runtime>;
	type Currency = Balances;
}

impl pallet_networks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::networks::WeightInfo<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
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

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(2)]
	pub type Timestamp = pallet_timestamp;

	#[runtime::pallet_index(3)]
	pub type Babe = pallet_babe;

	#[runtime::pallet_index(4)]
	pub type Grandpa = pallet_grandpa;

	#[runtime::pallet_index(5)]
	pub type ImOnline = pallet_im_online;

	#[runtime::pallet_index(6)]
	type AuthorityDiscovery = pallet_authority_discovery;

	#[runtime::pallet_index(7)]
	pub type Offences = pallet_offences;

	#[runtime::pallet_index(8)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(9)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(10)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(11)]
	pub type Council = pallet_collective<Instance1>;

	#[runtime::pallet_index(12)]
	pub type VoterList = pallet_bags_list;

	#[runtime::pallet_index(13)]
	pub type Historical = pallet_session_historical;

	#[runtime::pallet_index(14)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase;

	#[runtime::pallet_index(15)]
	pub type TransactionPayment = pallet_transaction_payment;

	#[runtime::pallet_index(16)]
	pub type Utility = pallet_utility;

	#[runtime::pallet_index(17)]
	pub type Sudo = pallet_sudo;

	#[runtime::pallet_index(18)]
	pub type Treasury = pallet_treasury;

	#[runtime::pallet_index(19)]
	pub type Members = pallet_members;

	#[runtime::pallet_index(20)]
	pub type Shards = pallet_shards;

	#[runtime::pallet_index(21)]
	pub type Elections = pallet_elections;

	#[runtime::pallet_index(22)]
	pub type Tasks = pallet_tasks;

	#[runtime::pallet_index(23)]
	pub type Timegraph = pallet_timegraph;

	#[runtime::pallet_index(24)]
	pub type Networks = pallet_networks;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
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

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	polkadot_sdk::frame_benchmarking::define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_babe, Babe]
		[pallet_bags_list, VoterList]
		[pallet_balances, Balances]
		[pallet_elections, Elections]
		[pallet_election_provider_multi_phase, ElectionProviderMultiPhase]
		[pallet_election_provider_support_benchmarking, EPSBench::<Runtime>]
		[pallet_grandpa, Grandpa]
		[pallet_im_online, ImOnline]
		[pallet_members, Members]
		[pallet_networks, Networks]
		[pallet_offences, OffencesBench::<Runtime>]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_shards, Shards]
		[pallet_staking, Staking]
		[pallet_sudo, Sudo]
		[pallet_tasks, Tasks]
		[pallet_timegraph, Timegraph]
		[pallet_timestamp, Timestamp]
		[pallet_utility, Utility]
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

		fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
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

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
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


	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> fg_primitives::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
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

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
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
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}

		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
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
		fn get_shard_tasks(shard_id: ShardId) -> Vec<TaskExecution> {
			Tasks::get_shard_tasks(shard_id)
		}

		fn get_task(task_id: TaskId) -> Option<TaskDescriptor>{
			Tasks::get_task(task_id)
		}

		fn get_task_signature(task_id: TaskId) -> Option<TssSignature> {
			Tasks::get_task_signature(task_id)
		}

		fn get_task_signer(task_id: TaskId) -> Option<PublicKey> {
			Tasks::get_task_signer(task_id)
		}

		fn get_task_hash(task_id: TaskId) -> Option<[u8; 32]> {
			Tasks::get_task_hash(task_id)
		}

		fn get_task_phase(task_id: TaskId) -> TaskPhase {
			Tasks::get_task_phase(task_id)
		}

		fn get_task_result(task_id: TaskId) -> Option<TaskResult>{
			Tasks::get_task_result(task_id)
		}

		fn get_task_shard(task_id: TaskId) -> Option<ShardId>{
			Tasks::get_task_shard(task_id)
		}

		fn get_gateway(network: NetworkId) -> Option<[u8; 20]> {
			Tasks::get_gateway(network)
		}
	}

	impl time_primitives::SubmitTransactionApi<Block> for Runtime {
		fn submit_transaction(encoded_transaction: Vec<u8>) -> Result<(), ()> {
			sp_io::offchain::submit_transaction(encoded_transaction)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList, list_benchmark};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmark!(list, extra, pallet_elections, Elections);
			list_benchmark!(list, extra, pallet_shards, Shards);
			list_benchmark!(list, extra, pallet_tasks, Tasks);
			list_benchmark!(list, extra, pallet_members, Members);
			list_benchmark!(list, extra, pallet_timegraph, Timegraph);
			list_benchmark!(list, extra, pallet_networks, Networks);
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{add_benchmark, baseline, Benchmarking, BenchmarkBatch};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}
			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}

			use frame_support::traits::{TrackedStorageKey, WhitelistedStorageKeys};
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmark!(params, batches, pallet_elections, Elections);
			add_benchmark!(params, batches, pallet_shards, Shards);
			add_benchmark!(params, batches, pallet_tasks, Tasks);
			add_benchmark!(params, batches, pallet_members, Members);
			add_benchmark!(params, batches, pallet_timegraph, Timegraph);
			add_benchmark!(params, batches, pallet_network, Networks);
			add_benchmarks!(params, batches);

			Ok(batches)
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
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
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
mod multiplier_tests {
	use super::*;
	use frame_support::{dispatch::DispatchInfo, traits::OnFinalize};
	use separator::Separatable;
	use sp_runtime::traits::Convert;

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
