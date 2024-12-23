//! This is the official timechain runtime.
//!
//! # Timechain Runtime
//!
//!
//! | Name    | Features | Profile |
//! |---------|----------|---------|
//! | mainnet | default  | mainnet |
//! | testnet | testnet  | testnet |
//! | develop | develop  | dev     |
//!
//! Until we can extract individual package config a bit better,
//! please check [`Runtime`] and the individual pallets.
//!
//! ## Frame Configuration
//!
//! |Section       |Pallet                |Config Implementation                               |
//! |--------------|----------------------|----------------------------------------------------|
//! |__Core__      |[`System`]            |[`Config`](struct@Runtime#config.System)            |
//! |              |[`Timestamp`]         |[`Config`](struct@Runtime#config.Timestamp)         |
//! |__Consensus__ |[`Authorship`]        |[`Config`](struct@Runtime#config.Authorship)|
//! |              |[`Session`]     |      [`Config`](struct@Runtime#config.Session)|
//! |              |[`Historical`]   |     [`Config`](struct@Runtime#config.Historical)|
//! |              |[`Babe`]              |[`Config`](struct@Runtime#config.Babe)              |
//! |              |[`Grandpa`]           |[`Config`](struct@Runtime#config.Grandpa)           |
//! |              |[`ImOnline`]          |[`Config`](struct@Runtime#config.ImOnline)          |
//! |              |[`AuthorityDiscovery`]|[`Config`](struct@Runtime#config.AuthorityDiscovery)|
//! |__Tokenomics__|[`Balances`]          |[`Config`](struct@Runtime#config.Balances)          |
//! |              |[`TransactionPayment`]|[`Config`](struct@Runtime#config.TransactionPayment)|
//! |              |[`Vesting`]        |[`Config`](struct@Runtime#config.Vesting)        |
//! |__Usability__|[`Utility`]          |[`Config`](struct@Runtime#config.Utility)          |
//! |             |[`Proxy`]          |[`Config`](struct@Runtime#config.Proxy)          |
//! |             |[`Multisig`]          |[`Config`](struct@Runtime#config.Multisig)          |
//!
//! ### Nominated Proof of Stake
//! - [`ElectionProviderMultiPhase`]
//! - [`Staking`]
//! - [`VoterList`]
//! - [`Offences`]
//!
//! ### On-chain services
//!  - [`Identity`]
//!  - [`Preimage`]
//!  - [`Scheduler`]
//!
//! ### On-chain governance
//!  - [`TechnicalCommittee`]
//!  - [`TechnicalMembership`]
//!  - [`SafeMode`]
//!
//! ### On-chain funding
//!  - [`Treasury`]
//!
//! ### Custom pallets
//!  - [`Governance`]
//!  - [`Members`]
//!  - [`Shards`]
//!  - [`Elections`]
//!  - [`Tasks`]
//!  - [`Timegraph`]
//!  - [`Networks`]
//!  - [`Dmail`]
//!
//! ## Weights and Fees
//!
//! ## Governance
//!
//! The main body of governance is responsible for maintaining the chain and
//! keeping it operational.
//!
//! ### Technical Committee
//!
//! The technical committee is managed using the [`pallet_collective`], [`pallet_membership`] and our own custom [`pallet_governance`].
//!
//! While the first two pallets tally the votes and manage the membership of the committee, our custom pallet it used to elevate committee origin to more privileged level for selected calls.
//!
//! Can be compiled with `#[no_std]`, ready for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limits.
#![recursion_limit = "1024"]
// Allow more readable constants
#![allow(clippy::identity_op)]

/// The runtime is split into it components
pub mod apis;
pub mod configs;

pub use apis::RuntimeApi;
pub use configs::consensus::SessionKeys;
pub use configs::core::{RuntimeBlockLength, RuntimeBlockWeights};
pub use configs::governance::{
	EnsureRootOrHalfTechnical, TechnicalMember, TechnicalQualifiedMajority, TechnicalSuperMajority,
	TechnicalUnanimity,
};
pub use configs::tokenomics::ExistentialDeposit;

/// Validator Set Bootstraping
mod validator_manager;

/// Helpers to handle variant flags
pub mod variants;

/// Import variant constants and macros
pub use variants::*;

/// Runtime test suite
#[cfg(test)]
mod tests;

/// Benchmarked pallet weights
mod weights;
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight};

/// Automatically generated nomination bag boundaries
mod staking_bags;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

// Import all substrate dependencies
use polkadot_sdk::*;

use frame_support::{
	parameter_types,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pallet_session::historical as pallet_session_historical;
// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};

use sp_runtime::{
	create_runtime_str, generic,
	traits::{Block as BlockT, Extrinsic, OpaqueKeys},
};
use sp_std::prelude::*;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use time_primitives::{
	AccountId, Balance, BatchId, BlockHash, BlockNumber, ChainName, ChainNetwork, Commitment,
	ErrorMsg, Gateway, GatewayMessage, Header, MemberStatus, MembersInterface, Moment, NetworkId,
	NetworksInterface, Nonce, PeerId, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, Signature,
	Task, TaskId, TaskResult, TssPublicKey, TssSignature, ANLOG, MICROANLOG, MILLIANLOG,
	SS58_PREFIX,
};

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

/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

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

/// Type shorthand for the balance type used to charge transaction fees
pub type PaymentBalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as pallet_transaction_payment::OnChargeTransaction<T>>::Balance;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type RuntimeSignedExtra = SignedExtra<Runtime>;

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, RuntimeSignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, RuntimeSignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, RuntimeSignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

/// Max size for serialized extrinsic params for this testing runtime.
/// This is a quite arbitrary but empirically battle tested value.
#[cfg(test)]
pub const CALL_PARAMS_MAX_SIZE: usize = 244;

/// Mainnet runtime version
#[cfg(not(any(feature = "testnet", feature = "develop")))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-timechain"),
	impl_name: create_runtime_str!("analog-timechain"),
	authoring_version: 0,
	spec_version: 0,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Testnet runtime version.
#[cfg(all(feature = "testnet", not(feature = "develop")))]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-testnet"),
	impl_name: create_runtime_str!("analog-testnet"),
	authoring_version: 0,
	spec_version: 0,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// Development runtime version.
#[cfg(feature = "develop")]
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("analog-develop"),
	impl_name: create_runtime_str!("analog-develop"),
	authoring_version: 0,
	spec_version: 0,
	impl_version: 0,
	apis: apis::RUNTIME_API_VERSIONS,
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

pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 2 * HOURS;
pub const EPOCH_DURATION_IN_SLOTS: u64 = {
	const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

	(EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
};

/// TODO: 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

/// Shared default babe genesis config
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

//use polkadot_sdk::*;

//pub use time_primitives::currency::*;

pub const TOTAL_ISSUANCE: Balance = 90_570_710 * ANLOG;

pub const TRANSACTION_BYTE_FEE: Balance = MICROANLOG;
pub const STORAGE_BYTE_FEE: Balance = 300 * MILLIANLOG; // Change based on benchmarking

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 750 * MILLIANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
}

parameter_types! {
	/// An epoch is a unit of time used for key operations in the consensus mechanism, such as validator
	/// rotations and randomness generation. Once set at genesis, this value cannot be changed without
	/// breaking block production.
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;

	/// This defines the interval at which new blocks are produced in the blockchain. It impacts
	/// the speed of transaction processing and finality, as well as the load on the network
	/// and validators. The value is defined in milliseconds.
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;

	/// The number of sessions that constitute an era. An era is the time period over which staking rewards
	/// are distributed, and validator set changes can occur. The era duration is a function of the number of
	/// sessions and the length of each session.
	pub const SessionsPerEra: sp_staking::SessionIndex = 6;

	/// The number of eras that a bonded stake must remain locked after the owner has requested to unbond.
	/// This value represents 21 days, assuming each era is 24 hours long. This delay is intended to increase
	/// network security by preventing stakers from immediately withdrawing funds after participating in staking.
	pub const BondingDuration: sp_staking::EraIndex = 24 * 21;

	/// The maximum number of validators a nominator can nominate. This sets an upper limit on how many validators
	/// can be supported by a single nominator. A higher number allows more decentralization but increases the
	/// complexity of the staking system.
	pub const MaxNominators: u32 = main_or_test!(0, 16);
}

pub type TechnicalCollective = pallet_collective::Instance1;

/// Mainnet runtime assembly
#[cfg(not(feature = "testnet"))]
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
	/// Base pallet mandatory for all frame runtimes.
	/// Current configuration can be found here [here](struct@Runtime#config.System).
	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	/// Simple timestamp extension.
	/// Current configuration can be found here [here](struct@Runtime#config.Timestamp).
	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	// Block production, finality, heartbeat and discovery
	/// Authorship tracking extension.
	/// Current configuration can be found here [here](struct@Runtime#config.Authorship).
	#[runtime::pallet_index(2)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(3)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(4)]
	pub type Historical = pallet_session_historical;

	/// Blind Assignment for Blockchain Extension block production.
	/// Current configuration can be found here [here](struct@Runtime#config.Babe).
	#[runtime::pallet_index(5)]
	pub type Babe = pallet_babe;

	/// GHOST-based Recursive Ancestor Deriving Prefix Agreement finality gadget.
	/// Current configuration can be found here [here](struct@Runtime#config.Grandpa).
	#[runtime::pallet_index(6)]
	pub type Grandpa = pallet_grandpa;

	/// Validator heartbeat protocol.
	/// Current configuration can be found here [here](struct@Runtime#config.ImOnline).
	#[runtime::pallet_index(7)]
	pub type ImOnline = pallet_im_online;

	/// Validator peer-to-peer discovery.
	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(8)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	// Tokens, fees and vesting
	/// Current configuration can be found here [here](struct@Runtime#config.Balances).
	#[runtime::pallet_index(9)]
	pub type Balances = pallet_balances;

	/// Current configuration can be found here [here](struct@Runtime#config.TransactionPayment).
	#[runtime::pallet_index(10)]
	pub type TransactionPayment = pallet_transaction_payment;

	/// Current configuration can be found here [here](struct@Runtime#config.Vesting).
	#[runtime::pallet_index(11)]
	pub type Vesting = pallet_vesting;

	// Batch, proxy and multisig support
	/// Current configuration can be found here [here](struct@Runtime#config.Utility).
	#[runtime::pallet_index(12)]
	pub type Utility = pallet_utility;

	/// Current configuration can be found here [here](struct@Runtime#config.Proxy).
	#[runtime::pallet_index(13)]
	pub type Proxy = pallet_proxy;

	/// Current configuration can be found here [here](struct@Runtime#config.Multisig).
	#[runtime::pallet_index(14)]
	pub type Multisig = pallet_multisig;

	// On-chain governance
	#[runtime::pallet_index(22)]
	pub type TechnicalCommittee = pallet_collective<Instance1>;

	#[runtime::pallet_index(23)]
	pub type TechnicalMembership = pallet_membership;

	#[runtime::pallet_index(24)]
	pub type SafeMode = pallet_safe_mode;

	#[runtime::pallet_index(25)]
	pub type ValidatorManager = validator_manager;

	// On-chain funding
	#[runtime::pallet_index(27)]
	pub type Treasury = pallet_treasury;

	// Custom governance
	#[runtime::pallet_index(32)]
	pub type Governance = pallet_governance;
}

/// Testnet and develop runtime assembly
#[cfg(feature = "testnet")]
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
	/// Base pallet mandatory for all frame runtimes.
	/// Current configuration can be found here [here](struct@Runtime#config.System).
	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	/// Simple timestamp extension.
	/// Current configuration can be found here [here](struct@Runtime#config.Timestamp).
	#[runtime::pallet_index(1)]
	pub type Timestamp = pallet_timestamp;

	// Block production, finality, heartbeat and discovery
	#[runtime::pallet_index(2)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(3)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(4)]
	pub type Historical = pallet_session_historical;

	/// Blind Assignment for Blockchain Extension block production.
	/// Current configuration can be found here [here](struct@Runtime#config.Babe).
	#[runtime::pallet_index(5)]
	pub type Babe = pallet_babe;

	/// GHOST-based Recursive Ancestor Deriving Prefix Agreement finality gadget.
	/// Current configuration can be found here [here](struct@Runtime#config.Grandpa).
	#[runtime::pallet_index(6)]
	pub type Grandpa = pallet_grandpa;

	/// Validator heartbeat protocol.
	/// Current configuration can be found here [here](struct@Runtime#config.ImOnline).
	#[runtime::pallet_index(7)]
	pub type ImOnline = pallet_im_online;

	/// Validator peer-to-peer discovery.
	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(8)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	// Tokens, fees and vesting
	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(9)]
	pub type Balances = pallet_balances;

	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(10)]
	pub type TransactionPayment = pallet_transaction_payment;

	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(11)]
	pub type Vesting = pallet_vesting;

	// Batch, proxy and multisig support
	/// Current configuration can be found here [here](struct@Runtime#config.Utility).
	#[runtime::pallet_index(12)]
	pub type Utility = pallet_utility;

	/// Current configuration can be found here [here](struct@Runtime#config.Proxy).
	#[runtime::pallet_index(13)]
	pub type Proxy = pallet_proxy;

	/// Current configuration can be found here [here](struct@Runtime#config.Multisig).
	#[runtime::pallet_index(14)]
	pub type Multisig = pallet_multisig;

	// Nominated proof of stake
	#[runtime::pallet_index(15)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase;

	#[runtime::pallet_index(16)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(17)]
	pub type VoterList = pallet_bags_list<Instance1>;

	#[runtime::pallet_index(18)]
	pub type Offences = pallet_offences;

	// On-chain identity,storage and scheduler
	#[runtime::pallet_index(19)]
	pub type Identity = pallet_identity;

	#[runtime::pallet_index(20)]
	pub type Preimage = pallet_preimage;

	#[runtime::pallet_index(21)]
	pub type Scheduler = pallet_scheduler;

	// On-chain governance
	#[runtime::pallet_index(22)]
	pub type TechnicalCommittee = pallet_collective<Instance1>;

	#[runtime::pallet_index(23)]
	pub type TechnicalMembership = pallet_membership;

	// On-chain funding
	#[runtime::pallet_index(27)]
	pub type Treasury = pallet_treasury;

	// = Custom pallets =

	// Custom governance
	#[runtime::pallet_index(32)]
	pub type Governance = pallet_governance;

	// general message passing pallets
	#[runtime::pallet_index(33)]
	pub type Members = pallet_members;

	#[runtime::pallet_index(34)]
	pub type Shards = pallet_shards;

	#[runtime::pallet_index(35)]
	pub type Elections = pallet_elections;

	#[runtime::pallet_index(36)]
	pub type Tasks = pallet_tasks;

	#[runtime::pallet_index(37)]
	pub type Timegraph = pallet_timegraph;

	#[runtime::pallet_index(38)]
	pub type Networks = pallet_networks;

	#[runtime::pallet_index(39)]
	pub type Dmail = pallet_dmail;
}

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

#[cfg(test)]
mod included_tests {
	use super::*;

	use frame_system::offchain::CreateSignedTransaction;

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
		let try_elect_shard: Weight =
			<Runtime as pallet_elections::Config>::WeightInfo::try_elect_shards(1);
		assert!(
			try_elect_shard.all_lte(avg_on_initialize),
			"BUG: One shard election consumes more weight than available in on-initialize"
		);
		assert!(
			try_elect_shard
				.all_lte(<Runtime as pallet_elections::Config>::WeightInfo::try_elect_shards(2)),
			"BUG: 1 shard election consumes more weight than 2 shard elections"
		);
		let mut num_elections: u32 = 3;
		while <Runtime as pallet_elections::Config>::WeightInfo::try_elect_shards(num_elections)
			.all_lt(avg_on_initialize)
		{
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

	use super::{
		frame_support::weights::WeightToFee as WeightToFeeT, tokenomics::WeightToFee,
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

#[cfg(test)]
mod multiplier_tests {
	use super::*;
	use frame_support::{dispatch::DispatchInfo, traits::OnFinalize};
	use pallet_elections::WeightInfo as W4;
	use pallet_members::WeightInfo as W2;
	use pallet_tasks::WeightInfo;
	use separator::Separatable;
	use time_primitives::ON_INITIALIZE_BOUNDS;

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
		let try_elect_shard: Weight =
			<Runtime as pallet_elections::Config>::WeightInfo::try_elect_shards(1);
		assert!(
			try_elect_shard.all_lte(avg_on_initialize),
			"BUG: One shard election consumes more weight than available in on-initialize"
		);
		assert!(
			try_elect_shard
				.all_lte(<Runtime as pallet_elections::Config>::WeightInfo::try_elect_shards(2)),
			"BUG: 1 shard election consumes more weight than 2 shard elections"
		);
		let mut num_elections: u32 = 3;
		while <Runtime as pallet_elections::Config>::WeightInfo::try_elect_shards(num_elections)
			.all_lt(avg_on_initialize)
		{
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
