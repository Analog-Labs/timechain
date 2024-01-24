// @generated to prevent rustfmt reformat/check
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

#[cfg(test)]
mod tests;
mod weights;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
use frame_system::{limits::BlockWeights, EnsureRoot};

use frame_election_provider_support::{
	generate_solution_type, onchain, ExtendedBalance, SequentialPhragmen,
};
use frame_support::{
	dispatch::DispatchClass,
	traits::{EitherOfDiverse, Imbalance},
	weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

use codec::{Decode, Encode};
use frame_election_provider_support::bounds::{ElectionBounds, ElectionBoundsBuilder};
use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};
use pallet_grandpa::{
	fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_session::historical as pallet_session_historical;
pub use runtime_common::{
	currency::*,
	weights::{BlockExecutionWeight, ExtrinsicBaseWeight},
};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::FixedPointNumber;
use sp_runtime::{
	create_runtime_str,
	curve::PiecewiseLinear,
	generic::{self, Era},
	impl_opaque_keys,
	traits::{
		AccountIdLookup, AtLeast32BitUnsigned, BlakeTwo256, Block as BlockT, BlockNumberProvider,
		Header as HeaderT, NumberFor, OpaqueKeys,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, Percent, SaturatedConversion,
};

use frame_system::EnsureRootWithSuccess;
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
pub use time_primitives::{
	AccountId, ChainName, ChainNetwork, Commitment, MemberStatus, MemberStorage, NetworkId, PeerId,
	ProofOfKnowledge, PublicKey, ShardId, ShardStatus, Signature, TaskCycle, TaskDescriptor,
	TaskError, TaskExecution, TaskId, TaskPhase, TaskResult, TssPublicKey, TssSignature, TxError,
	TxResult,
};
// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime,
	pallet_prelude::Get,
	parameter_types,
	traits::{
		ConstU128, ConstU16, ConstU32, ConstU64, ConstU8, Currency, EnsureOrigin,
		KeyOwnerProofSystem, OnUnbalanced, Randomness, StorageInfo,
	},
	weights::{constants::RocksDbWeight, ConstantMultiplier, IdentityFee, Weight, WeightToFee},
	PalletId, StorageValue,
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
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

pub const THRESHOLDS: [u64; 200] = [
	10_000_000_000,
	11_131_723_507,
	12_391_526_824,
	13_793_905_044,
	15_354_993_703,
	17_092_754_435,
	19_027_181_634,
	21_180_532_507,
	23_577_583_160,
	26_245_913_670,
	29_216_225_417,
	32_522_694_326,
	36_203_364_094,
	40_300_583_912,
	44_861_495_728,
	49_938_576_656,
	55_590_242_767,
	61_881_521_217,
	68_884_798_439,
	76_680_653_006,
	85_358_782_760,
	95_019_036_859,
	105_772_564_622,
	117_743_094_401,
	131_068_357_174,
	145_901_671_259,
	162_413_706_368,
	180_794_447_305,
	201_255_379_901,
	224_031_924_337,
	249_386_143_848,
	277_609_759_981,
	309_027_509_097,
	344_000_878_735,
	382_932_266_827,
	426_269_611_626,
	474_511_545_609,
	528_213_132_664,
	587_992_254_562,
	654_536_720_209,
	728_612_179_460,
	811_070_932_564,
	902_861_736_593,
	1_005_040_721_687,
	1_118_783_542_717,
	1_245_398_906_179,
	1_386_343_627_960,
	1_543_239_395_225,
	1_717_891_425_287,
	1_912_309_236_147,
	2_128_729_767_682,
	2_369_643_119_512,
	2_637_821_201_686,
	2_936_349_627_828,
	3_268_663_217_709,
	3_638_585_517_729,
	4_050_372_794_022,
	4_508_763_004_364,
	5_019_030_312_352,
	5_587_045_771_074,
	6_219_344_874_498,
	6_923_202_753_807,
	7_706_717_883_882,
	8_578_905_263_043,
	9_549_800_138_161,
	10_630_573_468_586,
	11_833_660_457_397,
	13_172_903_628_838,
	14_663_712_098_160,
	16_323_238_866_411,
	18_170_578_180_087,
	20_226_985_226_447,
	22_516_120_692_255,
	25_064_322_999_817,
	27_900_911_352_605,
	31_058_523_077_268,
	34_573_489_143_434,
	38_486_252_181_966,
	42_841_831_811_331,
	47_690_342_626_046,
	53_087_570_807_094,
	59_095_615_988_698,
	65_783_605_766_662,
	73_228_491_069_308,
	81_515_931_542_404,
	90_741_281_135_191,
	101_010_685_227_495,
	112_442_301_921_293,
	125_167_661_548_718,
	139_333_180_038_781,
	155_101_843_555_358,
	172_655_083_789_626,
	192_194_865_483_744,
	213_946_010_204_502,
	238_158_783_103_893,
	265_111_772_429_462,
	295_115_094_915_607,
	328_513_963_936_552,
	365_692_661_475_578,
	407_078_959_611_349,
	453_149_042_394_237,
	504_432_984_742_966,
	561_520_851_400_862,
	625_069_486_125_324,
	695_810_069_225_823,
	774_556_530_406_243,
	862_214_913_708_369,
	959_793_802_308_039,
	1_068_415_923_109_985,
	1_189_331_064_661_951,
	1_323_930_457_019_515,
	1_473_762_779_014_021,
	1_640_551_977_100_649,
	1_826_217_100_807_404,
	2_032_894_383_008_501,
	2_262_961_819_074_188,
	2_519_066_527_700_738,
	2_804_155_208_229_882,
	3_121_508_044_894_685,
	3_474_776_448_088_622,
	3_868_025_066_902_796,
	4_305_778_556_320_752,
	4_793_073_637_166_665,
	5_335_517_047_800_242,
	5_939_350_054_341_159,
	6_611_520_261_667_250,
	7_359_761_551_432_161,
	8_192_683_066_856_378,
	9_119_868_268_136_230,
	10_151_985_198_186_376,
	11_300_909_227_415_580,
	12_579_859_689_817_292,
	14_003_551_982_487_792,
	15_588_366_878_604_342,
	17_352_539_001_951_086,
	19_316_366_631_550_092,
	21_502_445_250_375_680,
	23_935_927_525_325_748,
	26_644_812_709_737_600,
	29_660_268_798_266_784,
	33_016_991_140_790_860,
	36_753_601_641_491_664,
	40_913_093_136_236_104,
	45_543_324_061_189_736,
	50_697_569_104_240_168,
	56_435_132_174_936_472,
	62_822_028_745_677_552,
	69_931_745_415_056_768,
	77_846_085_432_775_824,
	86_656_109_914_600_688,
	96_463_185_576_826_656,
	107_380_151_045_315_664,
	119_532_615_158_469_088,
	133_060_402_202_199_856,
	148_119_160_705_543_712,
	164_882_154_307_451_552,
	183_542_255_300_186_560,
	204_314_163_786_713_728,
	227_436_877_985_347_776,
	253_176_444_104_585_088,
	281_829_017_427_734_464,
	313_724_269_827_691_328,
	349_229_182_918_168_832,
	388_752_270_484_770_624,
	432_748_278_778_513_664,
	481_723_418_752_617_984,
	536_241_190_443_833_600,
	596_928_866_512_693_376,
	664_484_709_541_257_600,
	739_686_006_129_409_280,
	823_398_010_228_713_984,
	916_583_898_614_395_264,
	1_020_315_853_041_475_584,
	1_135_787_396_594_579_584,
	1_264_327_126_171_442_688,
	1_407_413_999_103_859_968,
	1_566_694_349_801_462_272,
	1_744_000_832_209_069_824,
	1_941_373_506_026_471_680,
	2_161_083_309_305_266_176,
	2_405_658_187_494_662_656,
	2_677_912_179_572_818_944,
	2_980_977_795_924_034_048,
	3_318_342_060_496_414_208,
	3_693_886_631_935_247_360,
	4_111_932_465_319_354_368,
	4_577_289_528_371_127_808,
	5_095_312_144_166_932_480,
	5_671_960_597_112_134_656,
	6_313_869_711_009_142_784,
	7_028_425_188_266_614_784,
	7_823_848_588_596_424_704,
	8_709_291_924_949_524_480,
	9_694_942_965_096_232_960,
	10_792_142_450_433_898_496,
	12_013_514_580_722_579_456,
	13_373_112_266_084_982_784,
	14_886_578_817_516_689_408,
	16_571_327_936_291_497_984,
	18_446_744_073_709_551_615,
];

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

/// An index to a block.
pub type BlockNumber = u32;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Index = u32;

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

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub babe: Babe,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("timechain-node"),
	impl_name: create_runtime_str!("timechain-node"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 105,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
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

pub const MILLICENTS: Balance = 1_000_000_000;
pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
pub const DOLLARS: Balance = 100 * CENTS;
/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}
parameter_types! {
	// OnChain values are lower.
	pub MaxOnChainElectingVoters: u32 = 5000;
	pub MaxOnChainElectableTargets: u16 = 1250;
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
	// TODO
	type ValidatorIdOf = pallet_staking::StashOf<Self>;

	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	// type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
	type WeightInfo = ();
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
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Index;
	/// The index type for blocks.
	type Block = Block;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = weights::system::WeightInfo<Runtime>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
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
	pub EpochDuration: u64 = 8 * HOURS as u64;

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
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominations;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type WeightInfo = ();
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
	// TODO
	// type ReportUnresponsiveness = Offences;
	type ReportUnresponsiveness = ();
	type UnsignedPriority = ImOnlineUnsignedPriority;
	// TODO
	// type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
	type WeightInfo = ();
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

parameter_types! {
	// phase durations. 1/4 of the last session for each.
	// in testing: 1min or half of the session for each
	pub SignedPhase: u32 = 10;
	pub UnsignedPhase: u32 = 5;

	// signed config
	pub const SignedMaxSubmissions: u32 = 16;
	pub const SignedMaxRefunds: u32 = 16 / 4;
	// 40 DOTs fixed deposit..
	pub const SignedDepositBase: Balance = 10;
	// 0.01 DOT per KB of solution data.
	pub const SignedDepositByte: Balance = 10;
	// Each good submission will get 1 DOT as reward
	pub SignedRewardBase: Balance = 1000;
	pub BetterUnsignedThreshold: Perbill = Perbill::from_rational(5u32, 10_000);

	// 4 hour session, 1 hour unsigned phase, 32 offchain executions.
	pub OffchainRepeat: BlockNumber = UnsignedPhase::get() / 32;

	/// We take the top 22500 nominators as electing voters..
	pub const MaxElectingVoters: u32 = 22_500;
	/// ... and all of the validators as electable targets. Whilst this is the case, we cannot and
	/// shall not increase the size of the validator intentions.
	pub const MaxElectableTargets: u16 = u16::MAX;
}

generate_solution_type!(
	#[compact]
	pub struct NposCompactSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		// 3:2:1 staked : parachains : float.
		// while there's no parachains, then this is 75% staked : 25% float.
		ideal_stake: 0_750_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	// Six sessions in an era (24 hours).
	// TODO
	// // 28 eras for unbonding (28 days).

	pub const SessionsPerEra: sp_staking::SessionIndex = 1;//6;
	pub const BondingDuration: sp_staking::EraIndex = 2;//24 * 28;
	pub const SlashDeferDuration: sp_staking::EraIndex = 0;//24 * 7; // 1/4 the bonding duration.

	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
	// 16
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
	pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
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
pub struct ConvertCurve<T>(sp_std::marker::PhantomData<T>);
impl<Balance: AtLeast32BitUnsigned + Clone, T: Get<&'static PiecewiseLinear<'static>>>
	pallet_staking::EraPayout<Balance> for ConvertCurve<T>
{
	fn era_payout(
		total_staked: Balance,
		total_issuance: Balance,
		era_duration_millis: u64,
	) -> (Balance, Balance) {
		let (validator_payout, max_payout) = pallet_staking::inflation::compute_total_payout(
			T::get(),
			total_staked,
			total_issuance,
			// Duration of era; more than u64::MAX is rewarded as u64::MAX.
			era_duration_millis,
		);
		let rest = max_payout.clone().saturating_sub(validator_payout.clone());
		let session_active_validators = Session::validators();
		// 20 percent of total reward.
		let send_reward = Percent::from_percent(20) * max_payout;
		// reward distribution for validators/chronicle accounts.
		let length = session_active_validators.len().saturated_into::<u8>();
		if length != 0 {
			// get division percentage for each validator
			let total_percentage = 100u8;
			let fraction = Percent::from_percent(total_percentage.saturating_div(length));
			// reward share of each validator.
			let share = fraction * send_reward;

			session_active_validators.iter().for_each(|item| {
				let _resp =
					Balances::deposit_into_existing(item, share.clone().unique_saturated_into());
			});
		}
		let val_payout = Percent::from_percent(80) * validator_payout;
		let rest_payout = Percent::from_percent(80) * rest;
		// send rest for payout.
		(val_payout, rest_payout)
	}
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
	// type CurrencyToVote = U128CurrencyToVote;
	type CurrencyToVote = CurrencyToVote;
	type RewardRemainder = Treasury;
	type RuntimeEvent = RuntimeEvent;
	type Slash = Treasury; // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type SessionInterface = Self;
	type EraPayout = ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	// type VoterList = BagsList;
	type VoterList = VoterList;
	type MaxUnlockingChunks = ConstU32<32>;
	// type OnStakerSlash = NominationPools;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type HistoryDepth = frame_support::traits::ConstU32<84>;
	type TargetList = UseValidatorsMap<Self>;
	type EventListeners = ();
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
}

parameter_types! {
	pub const MinerMaxLength: u32 = 256;
	pub MinerMaxWeight: Weight = RuntimeBlockWeights::get().max_block;

	pub const SignedFixedDeposit: Balance = deposit(2, 0);
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);

	// Note: the EPM in this runtime runs the election on-chain. The election bounds must be
	// carefully set so that an election round fits in one block.
	pub ElectionBoundsMultiPhase: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(10_000.into()).targets_count(1_500.into()).build();
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(5_000.into()).targets_count(1_250.into()).build();
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
/// to ensure election snapshot will not run out of memory. For now, we set them to smaller values
/// since the staking is bounded and the weight pipeline takes hours for this single pallet.

pub struct BenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for BenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = UnsignedPhase;
	type BetterUnsignedThreshold = BetterUnsignedThreshold;
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
	pub const BagThresholds: &'static [u64] = &THRESHOLDS;
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
	type MaxHolds = ();
	type RuntimeHoldReason = ();
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
/// https://research.web3.foundation/en/latest/polkadot/overview/2-token-economics.html#-2.-slow-adjusting-mechanism
pub type SlowAdjustingFeeUpdate<R> = TargetedFeeAdjustment<
	R,
	TargetBlockFullness,
	AdjustmentVariable,
	MinimumMultiplier,
	MaximumMultiplier,
>;

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
		nonce: Index,
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
	pub const ProposalBondMinimum: Balance = DOLLARS;
	pub const SpendPeriod: BlockNumber = DAYS;
	pub const Burn: Permill = Permill::from_percent(50);
	pub const MaxBalance: Balance = Balance::max_value();
	pub const ScheduleFee: Balance = 1;
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
}

parameter_types! {
	pub IndexerReward: Balance = ANLOG;
}

impl pallet_members::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::members::WeightInfo<Runtime>;
	type Elections = Elections;
	type Currency = Balances;
	type MinStake = ConstU128<5>;
	type HeartbeatTimeout = ConstU32<50>;
}

// Set in elections::GenesisConfig in node/chain_spec
pub const SHARD_SIZE: u16 = 3;
pub const SHARD_THRESHOLD: u16 = 2;

impl pallet_elections::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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

impl pallet_tasks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::tasks::WeightInfo<Runtime>;
	type Shards = Shards;
	type MaxRetryCount = ConstU8<3>;
	type WritePhaseTimeout = ConstU32<10>;
}

impl pallet_timegraph::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::timegraph::WeightInfo<Runtime>;
	type Currency = Balances;
}

impl pallet_networks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// TODO fix weights
	type WeightInfo = ();
	type MaxBlockchainSize = ConstU32<32>;
	type MaxNameSize = ConstU32<32>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime
	{
		System: frame_system,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Babe: pallet_babe,
		Grandpa: pallet_grandpa,
		ImOnline: pallet_im_online,
		Offences: pallet_offences,
		Authorship: pallet_authorship,
		Session: pallet_session,
		Staking: pallet_staking,
		Council: pallet_collective::<Instance1>,
		VoterList: pallet_bags_list,
		Historical: pallet_session_historical,
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase,
		TransactionPayment: pallet_transaction_payment,
		Utility: pallet_utility,
		Sudo: pallet_sudo,
		Treasury: pallet_treasury,
		Members: pallet_members,
		Shards: pallet_shards,
		Elections: pallet_elections,
		Tasks: pallet_tasks::{Pallet, Call, Storage, Event<T>},
		Timegraph: pallet_timegraph,
		Networks: pallet_networks,
	}
);

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
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_members, Members]
		[pallet_shards, Shards]
		[pallet_tasks, Tasks]
		[pallet_timegraph, Timegraph]
		[pallet_networks, Networks]
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

		fn initialize_block(header: &<Block as BlockT>::Header) {
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
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
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
			use codec::Encode;

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

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
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

		fn get_heartbeat_timeout() -> u64 {
			Members::get_heartbeat_timeout().into()
		}

		fn get_min_stake() -> u128 {
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

		fn get_shard_status(shard_id: ShardId) -> ShardStatus<<<Block as BlockT>::Header as HeaderT>::Number> {
			Shards::get_shard_status(shard_id)
		}

		fn get_shard_commitment(shard_id: ShardId) -> Commitment {
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
		fn get_task_cycle(task_id: TaskId) -> TaskCycle{
			Tasks::get_task_cycle(task_id)
		}
		fn get_task_phase(task_id: TaskId) -> TaskPhase {
			Tasks::get_task_phase(task_id)
		}
		fn get_task_results(task_id: TaskId, cycle: Option<TaskCycle>) -> Vec<(TaskCycle, TaskResult)>{
			Tasks::get_task_results(task_id, cycle)
		}
		fn get_task_shard(task_id: TaskId) -> Option<ShardId>{
			Tasks::get_task_shard(task_id)
		}
		fn get_gateway(network: NetworkId) -> Option<Vec<u8>> {
			Tasks::get_gateway(network)
		}
	}

	impl time_primitives::BlockTimeApi<Block> for Runtime {
		fn get_block_time_in_msec() -> u64{
			MILLISECS_PER_BLOCK
		}
	}

	impl time_primitives::SubmitTransactionApi<Block> for Runtime {
		fn submit_transaction(encoded_transaction: Vec<u8>) -> TxResult {
			sp_io::offchain::submit_transaction(encoded_transaction).map_err(|_| TxError::TxPoolError)
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
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();
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
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};

			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::{TrackedStorageKey, WhitelistedStorageKeys};
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
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
