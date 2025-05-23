//! On-chain governance

use polkadot_sdk::*;

use frame_support::{parameter_types, traits::EitherOfDiverse, weights::Weight};
use frame_system::EnsureRoot;

use sp_runtime::Perbill;

use time_primitives::{AccountId, Balance, BlockNumber, ANLOG};

// Local module imports
use crate::{
	Runtime, RuntimeBlockWeights, RuntimeCall, RuntimeEvent, RuntimeOrigin, TechnicalCollective,
	TechnicalCommittee, DAYS, HOURS,
};

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;

	pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(75) * RuntimeBlockWeights::get().max_block;
}

impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type MaxMembers = TechnicalMaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = DefaultAdminOrigin;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type DisapproveOrigin = DefaultAdminOrigin;
	type KillOrigin = DefaultAdminOrigin;
	type Consideration = ();
}

// Limit to membership check in development mode
pub type TechnicalMember = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;

// Various voting percentages to use in governance
pub type TechnicalHalf =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 2>;
pub type TechnicalMajority =
	pallet_collective::EnsureProportionMoreThan<AccountId, TechnicalCollective, 1, 2>;
pub type TechnicalQualifiedMajority =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
pub type TechnicalSuperMajority =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 3, 4>;
pub type TechnicalUnanimity =
	pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;

// Combine with root origin to allow easier benchmarking
pub type EnsureRootOrTechnicalMember = EitherOfDiverse<EnsureRoot<AccountId>, TechnicalMember>;
pub type EnsureRootOrHalfTechnical = EitherOfDiverse<EnsureRoot<AccountId>, TechnicalHalf>;

/// Default admin origin on mainnet
#[cfg(not(any(feature = "testnet", feature = "develop")))]
pub type DefaultAdminOrigin = EnsureRootOrHalfTechnical;

/// Default admin origin on testnet or any development environment
#[cfg(any(feature = "testnet", feature = "develop"))]
pub type DefaultAdminOrigin = EnsureRootOrTechnicalMember;

impl pallet_membership::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = DefaultAdminOrigin;
	type RemoveOrigin = DefaultAdminOrigin;
	type SwapOrigin = DefaultAdminOrigin;
	type ResetOrigin = DefaultAdminOrigin;
	type PrimeOrigin = DefaultAdminOrigin;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
	type MaxMembers = TechnicalMaxMembers;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
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
	pub const ReleaseDelay: u32 = 7 * DAYS;
}

impl pallet_governance::Config for Runtime {
	/// Default admin origin for system related governance
	type SystemAdmin = DefaultAdminOrigin;
	/// Default admin origin for staking related governance
	type StakingAdmin = DefaultAdminOrigin;
	#[cfg(feature = "develop")]
	/// Default admin origin for balances related governance
	type BalancesAdmin = DefaultAdminOrigin;
}
