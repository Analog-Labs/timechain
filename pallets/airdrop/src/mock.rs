use crate as pallet_airdrop;
//use crate::Call as AirdropCall;

use super::*;

//use scale_codec::Encode;

use frame_support::{derive_impl, ord_parameter_types, parameter_types, traits::WithdrawReasons};
use sp_core::ConstU64;

use sp_runtime::{
	traits::{Identity, IdentityLookup},
	BuildStorage,
};

use time_primitives::AccountId;

pub use sp_keyring::{
	AccountKeyring::{Alice, Bob, Charlie},
	Ed25519Keyring::{Dave, Eve, Ferdie},
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vesting: pallet_vesting,
		Airdrop: pallet_airdrop,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type AccountId = AccountId;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type AccountData = pallet_balances::AccountData<u64>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 1;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = Identity;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 1;
}

parameter_types! {
	// Currently needs to match the prefix to be benchmarked with as we can not sign in wasm.
	pub RawPrefix: &'static [u8] = b"Airdrop TANLOG to the Testnet account: ";
}

ord_parameter_types! {
	pub const Six: u64 = 6;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type RawPrefix = RawPrefix;
	type MinimumBalance = ConstU64<500>;
	type WeightInfo = TestWeightInfo;
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// We use default for brevity, but you can configure as desired if needed.
	pallet_balances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_airdrop::GenesisConfig::<Test> {
		claims: vec![
			(Alice.into(), 1000),
			(Bob.into(), 2000),
			(Dave.into(), 3000),
			(Eve.into(), 4000),
		],
		vesting: vec![(Alice.into(), 50, 10, 1), (Dave.into(), 50, 10, 1)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

pub fn total_claims() -> u64 {
	1000 + 2000 + 3000 + 4000
}
