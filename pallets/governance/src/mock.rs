use crate as pallet_governance;

use polkadot_sdk::*;

use frame_support::{construct_runtime, derive_impl, ord_parameter_types, parameter_types};
use frame_system::{EnsureRoot, EnsureSignedBy};
use sp_runtime::BuildStorage;
use sp_staking::{EraIndex, SessionIndex};

type AccountId = u64;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		Staking: pallet_staking,
		Governance: pallet_governance,
	}
);

// Main mock accounts
ord_parameter_types! {
	pub const SystemAdmin: AccountId = 1;
	pub const StakingAdmin: AccountId = 2;
	pub const Other: AccountId = 3;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<AccountId>;
	type Block = Block;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig)]
impl pallet_timestamp::Config for Test {}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub static Ongoing: bool = false;
	pub static MaxWinners: u32 = 100;
}

pub struct MockElection;
impl frame_election_provider_support::ElectionProviderBase for MockElection {
	type AccountId = AccountId;
	type BlockNumber = u64;
	type MaxWinners = MaxWinners;
	type DataProvider = Staking;
	type Error = ();
}

impl frame_election_provider_support::ElectionProvider for MockElection {
	fn ongoing() -> bool {
		Ongoing::get()
	}
	fn elect() -> Result<frame_election_provider_support::BoundedSupportsOf<Self>, Self::Error> {
		Err(())
	}
}

parameter_types! {
	pub static SessionsPerEra: SessionIndex = 1;
	pub static SlashDeferDuration: EraIndex = 0;
	pub static BondingDuration: EraIndex = 1;

	pub static MaxExposurePageSize: u32 = 64;
	pub static MaxUnlockingChunks: u32 = 32;
	pub static HistoryDepth: u32 = 0;
	pub static MaxControllersInDeprecationBatch: u32 = 1000;
}

impl pallet_staking::Config for Test {
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = ();
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EnsureRoot<AccountId>;
	type BondingDuration = BondingDuration;
	type SessionInterface = ();
	type EraPayout = ();
	type NextNewSession = ();
	type MaxExposurePageSize = MaxExposurePageSize;
	type ElectionProvider = MockElection;
	type GenesisElectionProvider = Self::ElectionProvider;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type MaxUnlockingChunks = MaxUnlockingChunks;
	type HistoryDepth = HistoryDepth;
	type MaxControllersInDeprecationBatch = MaxControllersInDeprecationBatch;
	type EventListeners = ();
	type BenchmarkingConfig = pallet_staking::TestBenchmarkingConfig;
	type WeightInfo = ();
	type DisablingStrategy = pallet_staking::UpToLimitDisablingStrategy;
}

impl pallet_governance::Config for Test {
	type SystemAdmin = EnsureSignedBy<SystemAdmin, AccountId>;
	//type StakingAdmin = EnsureSignedBy<StakingAdmin, AccountId>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
