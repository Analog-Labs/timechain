//! Consensus configuration

use polkadot_sdk::*;

use sp_std::prelude::*;

use frame_support::parameter_types;

use sp_runtime::{impl_opaque_keys, traits::OpaqueKeys, transaction_validity::TransactionPriority};

// Local module imports
#[cfg(not(feature = "testnet"))]
use crate::ValidatorManager;
use crate::{
	weights, AccountId, AuthorityDiscovery, Babe, BondingDuration, EpochDuration,
	ExpectedBlockTime, Grandpa, Historical, ImOnline, MaxAuthorities, MaxNominators, Runtime,
	RuntimeEvent, SessionsPerEra,
};
#[cfg(feature = "testnet")]
use crate::{Offences, Staking, Balance, Session};
#[cfg(feature = "testnet")]
use frame_support::traits::KeyOwnerProofSystem;
#[cfg(feature = "testnet")]
use sp_core::crypto::KeyTypeId;

/// ## <a id="config.Authorship">[`Authorship`] Config</a>
///
/// Tracks block authorship
impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	#[cfg(not(feature = "testnet"))]
	type EventHandler = ImOnline;
	#[cfg(feature = "testnet")]
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

/// Without staking, there are not controllers
pub struct IdentityValidator;
impl<T> sp_runtime::traits::Convert<T, Option<T>> for IdentityValidator {
	fn convert(t: T) -> Option<T> {
		Some(t)
	}
}

/// ## <a id="config.Session">[`Session`] Config</a>
///
/// Tracks session keys
impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	#[cfg(not(feature = "testnet"))]
	type ValidatorIdOf = IdentityValidator;
	#[cfg(feature = "testnet")]
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	#[cfg(not(feature = "testnet"))]
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, ValidatorManager>;
	#[cfg(feature = "testnet")]
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

pub struct FullIdentificationOf;
impl sp_runtime::traits::Convert<AccountId, Option<()>> for FullIdentificationOf {
	fn convert(_: AccountId) -> Option<()> {
		Some(())
	}
}

/// ## <a id="config.Historical">[`Historical`] Config</a>
///
/// Tracks historical session
#[cfg(not(feature = "testnet"))]
impl pallet_session::historical::Config for Runtime {
	type FullIdentification = ();
	type FullIdentificationOf = FullIdentificationOf;
}

#[cfg(feature = "testnet")]
impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
	/// This defines how long a misbehavior reports in the staking or consensus system
	/// remains valid before it expires. It is based on the bonding
	/// duration, sessions per era, and the epoch duration. The longer the bonding duration or
	/// number of sessions per era, the longer reports remain valid.
	pub const ReportLongevity: u64 =
		BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();
}

/// ## <a id="config.Babe">[`Babe`] Config</a>
///
/// babe block production
impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	#[cfg(not(feature = "testnet"))]
	type DisabledValidators = ();
	#[cfg(feature = "testnet")]
	type DisabledValidators = Session;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	#[cfg(not(feature = "testnet"))]
	type KeyOwnerProof = sp_core::Void;
	#[cfg(feature = "testnet")]
	type KeyOwnerProof =
		<Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	#[cfg(not(feature = "testnet"))]
	type EquivocationReportSystem = ();
	#[cfg(feature = "testnet")]
	type EquivocationReportSystem =
		pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

/// ## <a id="config.Grandpa">[`Grandpa`] Config</a>
///
/// grandpa finality gadget
impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = MaxAuthorities;
	type MaxNominators = MaxNominators;
	type MaxSetIdSessionEntries = MaxSetIdSessionEntries;
	#[cfg(not(feature = "testnet"))]
	type KeyOwnerProof = sp_core::Void;
	#[cfg(feature = "testnet")]
	type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, sp_consensus_grandpa::AuthorityId)>>::Proof;
	#[cfg(not(feature = "testnet"))]
	type EquivocationReportSystem = ();
	#[cfg(feature = "testnet")]
	type EquivocationReportSystem =
		pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::MAX;
	/// We prioritize im-online heartbeats over election solution submission.
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

/// ## <a id="config.ImOnline">[`ImOnline`] Config</a>
///
/// validator heartbeats
impl pallet_im_online::Config for Runtime {
	type AuthorityId = pallet_im_online::sr25519::AuthorityId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = Babe;
	type ValidatorSet = Historical;
	#[cfg(not(feature = "testnet"))]
	type ReportUnresponsiveness = ();
	#[cfg(feature = "testnet")]
	type ReportUnresponsiveness = Offences;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = weights::pallet_im_online::WeightInfo<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

/// ## <a id="config.AuthorityDiscovery">[`AuthorityDiscovery`] Config</a>
///
/// Add validator peer discovery, takes minimal configuration
impl pallet_authority_discovery::Config for Runtime {
	type MaxAuthorities = MaxAuthorities;
}
