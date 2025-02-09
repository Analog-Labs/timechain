//! Nominated Proof of Stake Config

use scale_codec::Decode;

use polkadot_sdk::*;

use frame_election_provider_support::{
	bounds::{ElectionBounds, ElectionBoundsBuilder},
	onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen, VoteWeight,
};
use frame_support::{
	dispatch::DispatchClass,
	pallet_prelude::Get,
	parameter_types,
	//traits::tokens::imbalance::ResolveTo,
	traits::{ConstU32, Currency, ExistenceRequirement, OnUnbalanced, WithdrawReasons},
	weights::Weight,
	PalletId,
};

use sp_runtime::{
	curve::PiecewiseLinear, traits::AccountIdConversion, transaction_validity::TransactionPriority,
	FixedU128, Perbill, Percent,
};
use sp_std::prelude::*;

use pallet_election_provider_multi_phase::{GeometricDepositBase, SolutionAccuracyOf};

use time_primitives::BlockNumber;
// Local module imports
use crate::{
	deposit, weights, AccountId, Balance, Balances, BlockExecutionWeight, BondingDuration,
	DelegatedStaking, ElectionProviderMultiPhase, EnsureRootOrHalfTechnical, EpochDuration,
	NegativeImbalance, NominationPools, PositiveImbalance, Runtime, RuntimeBlockLength,
	RuntimeBlockWeights, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, Session,
	SessionsPerEra, Staking, Timestamp, TransactionPayment, VoterList, ANLOG,
};

parameter_types! {
	/// This phase determines the time window, in blocks, during which signed
	/// transactions can be submitted. It is calculated as the duration of one
	/// the four epochs during an era.
	pub const SignedPhase: u32 = EpochDuration::get() as u32;
	/// This phase determines the time window, in blocks, during which unsigned
	/// off-chain transactions can be submitted. Like the signed phase, it is
	/// calculated as the duration of one the four epochs during an era.
	pub const UnsignedPhase: u32 = EpochDuration::get() as u32;

	// Signed Config
	/// This represents the fixed reward given to participants for submitting valid signed
	/// transactions. It is set to 1 ANLOG token, meaning that any participant who successfully
	/// submits a signed transaction will receive this base reward.
	pub const SignedRewardBase: Balance = 100 * ANLOG;
	/// This deposit ensures that users have economic stakes in the submission of valid signed
	/// transactions. It is currently set to 1 ANLOG, meaning participants must lock 1 ANLOG as
	pub const SignedFixedDeposit: Balance = 100 * ANLOG;
	/// This percentage increase applies to deposits for multiple or repeated signed transactions.
	/// It is set to 10%, meaning that each additional submission after the first will increase the
	/// required deposit by 10%. This serves as a disincentive to spamming the system with repeated
	/// submissions.
	pub const SignedDepositIncreaseFactor: Percent = Percent::from_percent(10);
	/// This deposit ensures that larger signed transactions incur higher costs, reflecting the
	/// increased resource consumption they require. It is set to 10 milliANLOG per byte.
	pub const SignedDepositByte: Balance = deposit(0, 1);

	// Miner Configs
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::MAX / 2;
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
		.voters_count(10_000.into()).targets_count(1_000.into()).build();
	pub ElectionBoundsOnChain: ElectionBounds = ElectionBoundsBuilder::default()
		.voters_count(5_000.into()).targets_count(1_000.into()).build();

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

/// ## <a id="config.ElectionProviderMultiPhase">[`ElectionProviderMultiPhase`] Config</a>
///
/// Manages off- and on-chain validator election
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

/// Virtual reward pool wallet
pub struct RewardPool;
impl RewardPool {
	/// Return internal virtual wallet id
	fn account_id() -> AccountId {
		PalletId(*b"timerwrd").into_account_truncating()
	}
}

impl OnUnbalanced<PositiveImbalance> for RewardPool {
	/// Take rewards from special rewards wallet, otherwise mint it via drop
	fn on_nonzero_unbalanced(imbalance: PositiveImbalance) {
		let _ = Balances::settle(
			&Self::account_id(),
			imbalance,
			WithdrawReasons::TRANSFER,
			ExistenceRequirement::AllowDeath,
		);
	}
}

/// Additional wrapper around reward pool to return funds
pub struct ToRewardPool;
impl OnUnbalanced<NegativeImbalance> for ToRewardPool {
	/// Return unspent reward to reward pool
	fn on_nonzero_unbalanced(imbalance: NegativeImbalance) {
		Balances::resolve_creating(&RewardPool::account_id(), imbalance);
	}
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_030_000,
		max_inflation: 0_080_000,
		ideal_stake: 0_600_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	/// The number of eras after a slashing event before the slashing is enacted. This delay allows participants
	/// to challenge slashes or react to slashing events. It is set to 1/3 of the BondingDuration.
	pub const SlashDeferDuration: sp_staking::EraIndex = 2 * 7;

	/// The reward curve used for staking payouts. This curve defines how rewards are distributed across validators
	/// and nominators, typically favoring higher stakes but ensuring diminishing returns as stakes increase.
	/// The curve is piecewise linear, allowing for different reward distribution models.
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;

	/// The maximum number of controllers that can be included in a deprecation batch when deprecated staking controllers
	/// are being phased out. This helps manage the process of retiring controllers to prevent overwhelming the system
	/// during upgrades.
	pub const MaxControllersInDeprecationBatch: u32 = 4096;

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

/// Configuration of benchmarking bounds
pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

/// ## <a id="config.Staking">[`Staking`] Config</a>
///
/// Tracks nominations and stake
impl pallet_staking::Config for Runtime {
	type Currency = Balances;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type RewardRemainder = ToRewardPool;
	type RuntimeEvent = RuntimeEvent;
	type Slash = (); //Treasury; // send the slashed funds to the treasury.
	/// Pay rewards from reward pool, otherwise mint them.
	type Reward = RewardPool;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A majority of the council can cancel the slash.
	type AdminOrigin = EnsureRootOrHalfTechnical;
	type SessionInterface = Self;
	/// Inflation curve that optimizes returned rewards
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type MaxExposurePageSize = ConstU32<256>;
	type NextNewSession = Session;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type VoterList = VoterList;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
	type MaxUnlockingChunks = ConstU32<32>;
	type HistoryDepth = HistoryDepth;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type EventListeners = NominationPools;
	type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}
parameter_types! {
	pub const BagThresholds: &'static [u64] = &crate::staking_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;

/// ## <a id="config.VoterList">[`VoterList`] Config</a>
///
/// Organizes nominations into bags by relative size
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	/// The voter bags-list is loosely kept up to date, and the real source of truth for the score
	/// of each node is the staking pallet.
	type ScoreProvider = Staking;
	type BagThresholds = BagThresholds;
	type Score = VoteWeight;
	type WeightInfo = weights::pallet_bags_list::WeightInfo<Runtime>;
}

/// ## <a id="config.Offences">`Offences` Config</a>
///
/// Tracks offences
impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
}

parameter_types! {
	pub const PostUnbondPoolsWindow: u32 = 4;
	pub const NominationPoolsPalletId: PalletId = PalletId(*b"timenmpl");
	pub const MaxPointsToBalance: u8 = 10;
}

use sp_runtime::traits::Convert;
pub struct BalanceToU256;
impl Convert<Balance, sp_core::U256> for BalanceToU256 {
	fn convert(balance: Balance) -> sp_core::U256 {
		sp_core::U256::from(balance)
	}
}
pub struct U256ToBalance;
impl Convert<sp_core::U256, Balance> for U256ToBalance {
	fn convert(n: sp_core::U256) -> Balance {
		n.try_into().unwrap_or(Balance::MAX)
	}
}

impl pallet_nomination_pools::Config for Runtime {
	type WeightInfo = pallet_nomination_pools::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type StakeAdapter =
		pallet_nomination_pools::adapter::DelegateStake<Self, Staking, DelegatedStaking>;
	type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = <Self as pallet_staking::Config>::MaxUnlockingChunks;
	type PalletId = NominationPoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type AdminOrigin = EnsureRootOrHalfTechnical;
}

parameter_types! {
	pub const DelegatedStakingPalletId: PalletId = PalletId(*b"timedgsk");
	pub const SlashRewardFraction: Perbill = Perbill::from_percent(1);
}

impl pallet_delegated_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = DelegatedStakingPalletId;
	type Currency = Balances;
	// slashes are sent to the treasury.
	type OnSlash = (); //ResolveTo<TreasuryAccountId<Self>, Balances>;
	type SlashRewardFraction = SlashRewardFraction;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CoreStaking = Staking;
}

#[cfg(test)]
mod tests {
	use super::*;

	use frame_election_provider_support::NposSolution;
	//use frame_support::weights::Weight;
	use sp_runtime::UpperOf;

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
}
