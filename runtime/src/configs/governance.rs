//! On-chain governance

use polkadot_sdk::*;

use frame_support::{parameter_types, traits::EitherOfDiverse, weights::Weight};
use frame_system::EnsureRoot;

use sp_runtime::Perbill;

use time_primitives::{AccountId, Balance, BlockNumber, ANLOG};

#[cfg(not(feature = "testnet"))]
use frame_support::traits::Contains;
#[cfg(not(feature = "testnet"))]
use frame_system::EnsureRootWithSuccess;
#[cfg(not(feature = "testnet"))]
use sp_runtime::traits::ConstU32;

// Local module imports
#[cfg(not(feature = "testnet"))]
use crate::{Balances, RuntimeHoldReason, Vesting};
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
	type SetMembersOrigin = EnsureRoot<Self::AccountId>;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
}

// Limit membership check to development mode
pub type TechnicalMember = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;

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

pub type EnsureRootOrTechnicalMember = EitherOfDiverse<EnsureRoot<AccountId>, TechnicalMember>;
pub type EnsureRootOrHalfTechnical = EitherOfDiverse<EnsureRoot<AccountId>, TechnicalHalf>;

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

/// Calls that can bypass the safe-mode pallet.
pub struct SafeModeWhitelistedCalls;
#[cfg(not(feature = "testnet"))]
impl Contains<RuntimeCall> for SafeModeWhitelistedCalls {
	fn contains(call: &RuntimeCall) -> bool {
		// TODO: Allow inherents
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
	pub const ReleaseDelay: u32 = 7 * DAYS;
}

#[cfg(not(feature = "testnet"))]
impl pallet_safe_mode::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WhitelistedCalls = SafeModeWhitelistedCalls;
	type EnterDuration = EnterDuration;
	type EnterDepositAmount = EnterDepositAmount;
	type ExtendDuration = ExtendDuration;
	type ExtendDepositAmount = ExtendDepositAmount;
	// TODO: Tie properly into governance
	type ForceEnterOrigin = EnsureRootWithSuccess<AccountId, ConstU32<9>>;
	type ForceExtendOrigin = EnsureRootWithSuccess<AccountId, ConstU32<11>>;
	type ForceExitOrigin = EnsureRootOrHalfTechnical;
	type ForceDepositOrigin = EnsureRoot<AccountId>;
	type ReleaseDelay = ReleaseDelay;
	type Notify = ();
	type WeightInfo = pallet_safe_mode::weights::SubstrateWeight<Runtime>;
}

#[cfg(not(feature = "testnet"))]
impl pallet_governance::Config for Runtime {
	/// Default admin origin for system related governance
	type SystemAdmin = EnsureRootOrHalfTechnical;
	// Default admin origin for staking related governance
	//type StakingAdmin = EnsureRootOrHalfTechnical;
}

#[cfg(feature = "testnet")]
impl pallet_governance::Config for Runtime {
	/// Development admin origin for all system calls
	type SystemAdmin = EnsureRootOrTechnicalMember;
	// Development admin origin for all staking calls
	//type StakingAdmin = EnsureRootOrTechnicalMember;
}

#[cfg(not(feature = "testnet"))]
impl pallet_validators::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PrivilegedOrigin = EnsureRootOrHalfTechnical;
}
