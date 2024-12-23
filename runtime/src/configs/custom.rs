use polkadot_sdk::{sp_runtime::traits::Ensure, *};

use frame_support::{
	pallet_prelude::Get,
	parameter_types,
	traits::{ConstU128, ConstU32},
};

use sp_runtime::traits::{Block as BlockT, Extrinsic, OpaqueKeys};

// Can't use `FungibleAdapter` here until Treasury pallet migrates to fungibles
// <https://github.com/paritytech/polkadot-sdk/issues/226>
#[allow(deprecated)]
pub use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};

use time_primitives::ANLOG;
// Local module imports
use crate::{
	Balance, Balances, EnsureRootOrHalfTechnical, Runtime, RuntimeEvent, TechnicalMember,
	TechnicalQualifiedMajority, TechnicalSuperMajority, TechnicalUnanimity,
};
#[cfg(feature = "testnet")]
use crate::{Elections, Members, Networks, Shards, Tasks};

use super::governance::EnsureRootOrTechnicalMember;

// Custom pallet config
parameter_types! {
	pub IndexerReward: Balance = ANLOG;
}

#[cfg(not(feature = "testnet"))]
/// Default admin origin for all chronicle related pallets
type ChronicleAdmin = EnsureRootOrHalfTechnical;

#[cfg(feature = "testnet")]
/// Development admin origin for all chronicle related pallets
type ChronicleAdmin = EnsureRootOrTechnicalMember;

#[cfg(feature = "testnet")]
impl pallet_members::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_members::WeightInfo<Runtime>;
	type Elections = Elections;
	type Shards = Shards;
	type MinStake = ConstU128<0>;
	type HeartbeatTimeout = ConstU32<300>;
	type MaxTimeoutsPerBlock = ConstU32<25>;
}

#[cfg(feature = "testnet")]
impl pallet_elections::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::pallet_elections::WeightInfo<Runtime>;
	type Members = Members;
	type Shards = Shards;
	type Networks = Networks;
	type MaxElectionsPerBlock = ConstU32<25>;
}

#[cfg(feature = "testnet")]
impl pallet_shards::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::pallet_shards::WeightInfo<Runtime>;
	type Members = Members;
	type Elections = Elections;
	type Tasks = Tasks;
	type DkgTimeout = ConstU32<10>;
}

#[cfg(feature = "testnet")]
impl pallet_tasks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::pallet_tasks::WeightInfo<Runtime>;
	type Networks = Networks;
	type Shards = Shards;
	type MaxTasksPerBlock = ConstU32<50>;
	type MaxBatchesPerBlock = ConstU32<10>;
}

#[cfg(feature = "testnet")]
parameter_types! {
 pub const InitialRewardPoolAccount: AccountId = AccountId::new([0_u8; 32]);
 pub const InitialTimegraphAccount: AccountId = AccountId::new([0_u8; 32]);
 pub const InitialThreshold: Balance = 1000;
}

#[cfg(feature = "testnet")]
impl pallet_timegraph::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_timegraph::WeightInfo<Runtime>;
	type Currency = Balances;
	type InitialRewardPoolAccount = InitialRewardPoolAccount;
	type InitialTimegraphAccount = InitialTimegraphAccount;
	type InitialThreshold = InitialThreshold;
}

#[cfg(feature = "testnet")]
impl pallet_networks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = ChronicleAdmin;
	type WeightInfo = weights::pallet_networks::WeightInfo<Runtime>;
	type Tasks = Tasks;
}

#[cfg(feature = "testnet")]
impl pallet_dmail::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_dmail::WeightInfo<Runtime>;
}
