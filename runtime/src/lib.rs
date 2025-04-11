//! This is the official timechain runtime.
//!
//! # Timechain Runtime and Environments
//!
//! New code is first tested on development (develop), then integrated with all other projects on integration.
//!
//! Once ready for release, new features start long term testing on testnet (testnet). Once everybody is happy, including the community and external partners, features are brought to timechain (mainnet).
//!
//! The staging can be optionally used to test any mainnet specific migrations, features or other oddities.
//!
//! | Name    | Features         | Profile |
//! |---------|------------------|---------|
//! | mainnet | default          | mainnet |
//! | staging | develop          | testnet |
//! | testnet | testnet          | testnet |
//! | develop | testnet,develop  | testnet |
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
//! |__Consensus__ |[`Authorship`]        |[`Config`](struct@Runtime#config.Authorship)        |
//! |              |[`Session`]           |[`Config`](struct@Runtime#config.Session)           |
//! |              |[`Historical`]        |[`Config`](struct@Runtime#config.Historical)        |
//! |              |[`Babe`]              |[`Config`](struct@Runtime#config.Babe)              |
//! |              |[`Grandpa`]           |[`Config`](struct@Runtime#config.Grandpa)           |
//! |              |[`ImOnline`]          |[`Config`](struct@Runtime#config.ImOnline)          |
//! |              |[`AuthorityDiscovery`]|[`Config`](struct@Runtime#config.AuthorityDiscovery)|
//! |__Tokenomics__|[`Balances`]          |[`Config`](struct@Runtime#config.Balances)          |
//! |              |[`TransactionPayment`]|[`Config`](struct@Runtime#config.TransactionPayment)|
//! |              |[`Vesting`]           |[`Config`](struct@Runtime#config.Vesting)           |
//! |__Utilities__ |[`Utility`]           |[`Config`](struct@Runtime#config.Utility)           |
//! |              |[`Proxy`]             |[`Config`](struct@Runtime#config.Proxy)             |
//! |              |[`Multisig`]          |[`Config`](struct@Runtime#config.Multisig)          |
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
#![allow(non_local_definitions)]

// The runtime is split into its components
//pub mod apis;
pub mod configs;
pub mod offchain;
pub mod version;

/// Helpers to handle variant flags
pub mod variants;

pub use version::VERSION;

// The runtime configs and its sections
pub use configs::consensus::SessionKeys;
pub use configs::core::{
	BlockHashCount, RuntimeBlockLength, RuntimeBlockWeights, AVERAGE_ON_INITIALIZE_RATIO,
};
#[cfg(feature = "testnet")]
pub use configs::custom::PrevalidateFeeless;
pub use configs::governance::DefaultAdminOrigin;
pub use configs::tokenomics::{ExistentialDeposit, LengthToFee, WeightToFee};

/// Import variant constants and macros
pub use variants::*;

/// Runtime benchmark list
#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
mod benches;

/// Runtime test suite
#[cfg(test)]
mod tests;

/// Benchmarked pallet weights
mod weights;
pub use weights::{BlockExecutionWeight, ExtrinsicBaseWeight};

/// Automatically generated nomination bag boundaries
mod staking_bags;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

// Import all substrate dependencies
use polkadot_sdk::*;

use frame_support::{
	parameter_types,
	traits::Currency,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use pallet_session::historical as pallet_session_historical;

use frame_support::traits::KeyOwnerProofSystem;
use scale_codec::Encode;
#[cfg(feature = "runtime-benchmarks")]
use scale_info::prelude::string::String;
use sp_runtime::generic;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity};
use sp_runtime::KeyTypeId;
use sp_std::prelude::*;

pub use time_primitives::{
	AccountId, Balance, BatchId, BlockHash, BlockNumber, ChainName, Commitment, ErrorMsg,
	GatewayMessage, Header, MemberStatus, MembersInterface, Moment, NetworkId, NetworksInterface,
	Nonce, PeerId, ProofOfKnowledge, PublicKey, ShardId, ShardStatus, Signature, Task, TaskId,
	TaskResult, TssPublicKey, TssSignature, ANLOG, MICROANLOG, MILLIANLOG,
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
#[cfg(not(feature = "testnet"))]
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

/// Shared signing extensions
#[cfg(feature = "testnet")]
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
	PrevalidateFeeless<Runtime>,
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

// Useful types when handeling currency
pub type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;
pub type PositiveImbalance = <Balances as Currency<AccountId>>::PositiveImbalance;

/// Max size for serialized extrinsic params for this testing runtime.
/// This is a quite arbitrary but empirically battle tested value.
#[cfg(test)]
pub const CALL_PARAMS_MAX_SIZE: usize = 448;

/// Maximum block size
pub const MAX_BLOCK_LENGTH: u32 = 5 * 1024 * 1024;

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

/// TODO: 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

/// Shared default babe genesis config
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
	sp_consensus_babe::BabeEpochConfiguration {
		c: PRIMARY_PROBABILITY,
		allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryVRFSlots,
	};

/// TODO: Clean this up and move to tokenomics
pub const STORAGE_BYTE_FEE: Balance = 300 * MILLIANLOG; // Change based on benchmarking

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 750 * MILLIANLOG + (bytes as Balance) * STORAGE_BYTE_FEE
}

parameter_types! {
	/// An epoch is a unit of time used for key operations in the consensus mechanism, such as validator
	/// rotations and randomness generation. Once set at genesis, this value cannot be changed without
	/// breaking block production.
	pub const EpochDuration: u64 = main_test_or_dev!(3 * HOURS, 30 * MINUTES, 5 * MINUTES) as u64;

	/// This defines the interval at which new blocks are produced in the blockchain. It impacts
	/// the speed of transaction processing and finality, as well as the load on the network
	/// and validators. The value is defined in milliseconds.
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;

	/// The number of sessions that constitute an era. An era is the time period over which staking rewards
	/// are distributed, and validator set changes can occur. The era duration is a function of the number of
	/// sessions and the length of each session.
	pub const SessionsPerEra: sp_staking::SessionIndex = 4;

	/// The number of eras that a bonded stake must remain locked after the owner has requested to unbond.
	/// This value represents 21 days, assuming each era is 12 hours long. This delay is intended to increase
	/// network security by preventing stakers from immediately withdrawing funds after participating in staking.
	pub const BondingDuration: sp_staking::EraIndex = 2 * 21;

	/// The maximum number of validators a nominator can nominate. This sets an upper limit on how many validators
	/// can be supported by a single nominator. A higher number allows more decentralization but increases the
	/// complexity of the staking system.
	pub const MaxNominators: u32 = main_or_test!(0, 16);

	/// Maximum numbers of authorities
	pub const MaxAuthorities: u32 = 100;
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

	/// Blind Assignment for Blockchain Extension block production.
	/// Current configuration can be found here [here](struct@Runtime#config.Babe).
	#[runtime::pallet_index(2)]
	pub type Babe = pallet_babe;

	/// GHOST-based Recursive Ancestor Deriving Prefix Agreement finality gadget.
	/// Current configuration can be found here [here](struct@Runtime#config.Grandpa).
	#[runtime::pallet_index(3)]
	pub type Grandpa = pallet_grandpa;

	/// Validator heartbeat protocol.
	/// Current configuration can be found here [here](struct@Runtime#config.ImOnline).
	#[runtime::pallet_index(4)]
	pub type ImOnline = pallet_im_online;

	/// Validator peer-to-peer discovery.
	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(5)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	/// Authorship tracking extension.
	/// Current configuration can be found here [here](struct@Runtime#config.Authorship).
	#[runtime::pallet_index(6)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(7)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(8)]
	pub type Historical = pallet_session_historical;

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

	// Nominated proof of stake

	#[runtime::pallet_index(15)]
	pub type ElectionProviderMultiPhase = pallet_election_provider_multi_phase;

	#[runtime::pallet_index(16)]
	pub type Staking = pallet_staking;

	#[runtime::pallet_index(17)]
	pub type VoterList = pallet_bags_list<Instance1>;

	#[runtime::pallet_index(18)]
	pub type Offences = pallet_offences;

	#[runtime::pallet_index(28)]
	pub type NominationPools = pallet_nomination_pools;

	#[runtime::pallet_index(29)]
	pub type DelegatedStaking = pallet_delegated_staking;

	// On-chain governance

	#[runtime::pallet_index(22)]
	pub type TechnicalCommittee = pallet_collective<Instance1>;

	#[runtime::pallet_index(23)]
	pub type TechnicalMembership = pallet_membership;

	// Custom governance

	#[runtime::pallet_index(32)]
	pub type Governance = pallet_governance;

	// Custom funding pallets

	#[runtime::pallet_index(42)]
	pub type Airdrop = pallet_airdrop;

	#[runtime::pallet_index(43)]
	pub type Launch = pallet_launch;
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

	/// Blind Assignment for Blockchain Extension block production.
	/// Current configuration can be found here [here](struct@Runtime#config.Babe).
	#[runtime::pallet_index(2)]
	pub type Babe = pallet_babe;

	/// GHOST-based Recursive Ancestor Deriving Prefix Agreement finality gadget.
	/// Current configuration can be found here [here](struct@Runtime#config.Grandpa).
	#[runtime::pallet_index(3)]
	pub type Grandpa = pallet_grandpa;

	/// Validator heartbeat protocol.
	/// Current configuration can be found here [here](struct@Runtime#config.ImOnline).
	#[runtime::pallet_index(4)]
	pub type ImOnline = pallet_im_online;

	/// Validator peer-to-peer discovery.
	/// Current configuration can be found here [here](struct@Runtime#config.AuthorityDiscovery).
	#[runtime::pallet_index(5)]
	pub type AuthorityDiscovery = pallet_authority_discovery;

	#[runtime::pallet_index(6)]
	pub type Authorship = pallet_authorship;

	#[runtime::pallet_index(7)]
	pub type Session = pallet_session;

	#[runtime::pallet_index(8)]
	pub type Historical = pallet_session_historical;

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

	#[runtime::pallet_index(28)]
	pub type NominationPools = pallet_nomination_pools;

	#[runtime::pallet_index(29)]
	pub type DelegatedStaking = pallet_delegated_staking;

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

	// Smart Contracts

	#[runtime::pallet_index(50)]
	pub type Revive = pallet_revive;
}

// All migrations executed on runtime upgrade implementing `OnRuntimeUpgrade`.
type Migrations = (
	pallet_session::migrations::v1::MigrateV0ToV1<
		Runtime,
		pallet_session::migrations::v1::InitOffenceSeverity<Runtime>,
	>,
	pallet_staking::migrations::v16::MigrateV15ToV16<Runtime>,
);

#[cfg(test)]
mod core_tests {
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
}

sp_api::impl_runtime_apis! {

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
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

	impl sp_api::Core<Block> for Runtime {
		fn version() -> sp_version::RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as sp_runtime::traits::Block>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> sp_core::OpaqueMetadata {
			sp_core::OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<sp_core::OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
		fn authorities() -> Vec<sp_authority_discovery::AuthorityId> {
			AuthorityDiscovery::authorities()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as sp_runtime::traits::Block>::Extrinsic) -> sp_runtime::ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as sp_runtime::traits::Block>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as sp_runtime::traits::Block>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
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
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as sp_runtime::traits::Block>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof.decode()?,
			)
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
				<Block as sp_runtime::traits::Block>::Hash,
				sp_runtime::traits::NumberFor<Block>,
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
			authority_id: sp_consensus_grandpa::AuthorityId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			Historical::prove((sp_consensus_grandpa::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_grandpa::OpaqueKeyOwnershipProof::new)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as sp_runtime::traits::Block>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(uxt: <Block as sp_runtime::traits::Block>::Extrinsic, len: u32) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}

		fn query_fee_details(uxt: <Block as sp_runtime::traits::Block>::Extrinsic, len: u32) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}

		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}

		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_nomination_pools_runtime_api::NominationPoolsApi<Block, AccountId, Balance> for Runtime {
		fn pending_rewards(member: AccountId) -> Balance {
			NominationPools::api_pending_rewards(member).unwrap_or_default()
		}

		fn points_to_balance(pool_id: u32, points: Balance) -> Balance {
			NominationPools::api_points_to_balance(pool_id, points)
		}

		fn balance_to_points(pool_id: u32, balance: Balance) -> Balance {
			NominationPools::api_balance_to_points(pool_id, balance)
		}

		fn pool_pending_slash(pool_id: u32) -> Balance {
			NominationPools::api_pool_pending_slash(pool_id)
		}

		fn member_pending_slash(member: AccountId) -> Balance {
			NominationPools::api_member_pending_slash(member)
		}

		fn pool_needs_delegate_migration(pool_id: u32) -> bool {
			NominationPools::api_pool_needs_delegate_migration(pool_id)
		}

		fn member_needs_delegate_migration(member: AccountId) -> bool {
			NominationPools::api_member_needs_delegate_migration(member)
		}

		fn member_total_balance(member: AccountId) -> Balance {
			NominationPools::api_member_total_balance(member)
		}

		fn pool_balance(pool_id: u32) -> Balance {
			NominationPools::api_pool_balance(pool_id)
		}

		fn pool_accounts(pool_id: u32) -> (AccountId, AccountId) {
			NominationPools::api_pool_accounts(pool_id)
		}
	}

	impl pallet_staking_runtime_api::StakingApi<Block, Balance, AccountId> for Runtime {
		fn nominations_quota(balance: Balance) -> u32 {
			Staking::api_nominations_quota(balance)
		}

		fn eras_stakers_page_count(era: u32, account: AccountId) -> u32 {
			Staking::api_eras_stakers_page_count(era, account)
		}

		fn pending_rewards(era: u32, account: AccountId) -> bool {
			Staking::api_pending_rewards(era, account)
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::MembersApi<Block> for Runtime {
		fn get_member_peer_id(account: &AccountId) -> Option<time_primitives::PeerId> {
			Members::member_peer_id(account)
		}

		fn get_heartbeat_timeout() -> BlockNumber {
			Members::get_heartbeat_timeout()
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::NetworksApi<Block> for Runtime {
		fn get_network(network_id: NetworkId) -> Option<time_primitives::ChainName> {
			Networks::get_network(network_id)
		}

		fn get_gateway(network: NetworkId) -> Option<time_primitives::Address32> {
			Networks::gateway(network)
		}

		fn get_cctp_contracts(network: NetworkId) -> Option<time_primitives::CctpContracts> {
			Networks::get_cctp_contracts(network)
		}

		fn get_cctp_url(network: NetworkId) -> Option<time_primitives::CctpUrl> {
			Networks::get_cctp_url(network)
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::ShardsApi<Block> for Runtime {
		fn get_shards(account: &AccountId) -> Vec<time_primitives::ShardId> {
			Shards::get_shards(account)
		}

		fn get_shard_members(shard_id: time_primitives::ShardId) -> Vec<(AccountId, time_primitives::MemberStatus)> {
			Shards::get_shard_members(shard_id)
		}

		fn get_shard_threshold(shard_id: time_primitives::ShardId) -> u16 {
			Shards::get_shard_threshold(shard_id)
		}

		fn get_shard_status(shard_id: time_primitives::ShardId) -> time_primitives::ShardStatus {
			Shards::get_shard_status(shard_id)
		}

		fn get_shard_commitment(shard_id: time_primitives::ShardId) -> Option<time_primitives::Commitment> {
			Shards::get_shard_commitment(shard_id)
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::TasksApi<Block> for Runtime {
		fn get_shard_tasks(shard_id: time_primitives::ShardId) -> Vec<time_primitives::TaskId> {
			Tasks::get_shard_tasks(shard_id)
		}

		fn get_task(task_id: time_primitives::TaskId) -> Option<time_primitives::Task>{
			Tasks::get_task(task_id)
		}

		fn get_task_result(task_id: time_primitives::TaskId) -> Option<Result<(), time_primitives::ErrorMsg>>{
			Tasks::get_task_result(task_id)
		}

		fn get_task_shard(task_id: time_primitives::TaskId) -> Option<time_primitives::ShardId>{
			Tasks::get_task_shard(task_id)
		}

		fn get_batch_message(batch_id: time_primitives::BatchId) -> Option<time_primitives::GatewayMessage> {
			Tasks::get_batch_message(batch_id)
		}

		fn get_failed_tasks() -> Vec<time_primitives::TaskId> {
			Tasks::get_failed_tasks()
		}
	}

	#[cfg(feature = "testnet")]
	impl time_primitives::SubmitTransactionApi<Block> for Runtime {
		fn submit_transaction(encoded_transaction: Vec<u8>) -> Result<(), ()> {
			sp_io::offchain::submit_transaction(encoded_transaction)
		}
	}

	/// - __genesis-builder__: support generation of custom genesis
	#[cfg(feature = "genesis-builder")]
	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			use frame_support::genesis_builder_helper::build_state;
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			use frame_support::genesis_builder_helper::get_preset;
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			vec![]
		}
	}

	/// - __runtime-benchmarks__: support runtime benchmarking
	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_system_benchmarking::extensions::Pallet as SystemExtensionsBench;
			use baseline::Pallet as BaselineBench;

			// Import substrate macros created by macros (and all pallets by effects)
			use crate::*;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);
			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, String> {
			use frame_benchmarking::{baseline, BenchmarkBatch};

			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency
			// issues. To get around that, we separated the Session benchmarks into its own crate,
			// which is why we need these two lines below.
			use pallet_session_benchmarking::Pallet as SessionBench;
			use pallet_nomination_pools_benchmarking::Pallet as NominationPoolsBench;
			use pallet_offences_benchmarking::Pallet as OffencesBench;
			use pallet_election_provider_support_benchmarking::Pallet as EPSBench;
			use frame_system_benchmarking::Pallet as SystemBench;
			use frame_system_benchmarking::extensions::Pallet as SystemExtensionsBench;
			use baseline::Pallet as BaselineBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_nomination_pools_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
			impl pallet_election_provider_support_benchmarking::Config for Runtime {}
			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::{TrackedStorageKey, WhitelistedStorageKeys};
			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			// Import substrate macros created by macros (and all pallets by effects)
			use crate::*;

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);
			Ok(batches)
		}
	}

	/// - __try-runtime__: support try runtime testing
	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, crate::RuntimeBlockWeights::get().max_block)
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
}
