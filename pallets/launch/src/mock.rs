use crate as pallet_launch;

use polkadot_sdk::*;

use frame_support::{derive_impl, parameter_types, traits::WithdrawReasons, PalletId};
use frame_system::EnsureRoot;
use sp_core::ConstU128;
use sp_runtime::{
	traits::{ConvertInto, IdentityLookup},
	BuildStorage,
};

use time_primitives::{AccountId, Balance, ANLOG};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vesting: pallet_vesting,
		Airdrop: pallet_airdrop,
		Launch: pallet_launch,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type AccountId = AccountId;
	type AccountData = pallet_balances::AccountData<Balance>;
	type Lookup = IdentityLookup<Self::AccountId>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 1;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub RawPrefix: &'static [u8] = b"Airdrop DANLOG to the test account: ";
	pub LaunchId: PalletId = PalletId(*b"testlnch");
}

impl pallet_airdrop::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type RawPrefix = RawPrefix;
	// Use mainnet existential deposit to ensure appropriate testing
	type MinimumBalance = ConstU128<ANLOG>;
	type WeightInfo = pallet_airdrop::TestWeightInfo;
}

impl pallet_launch::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LaunchId;
	// Use mainnet existential deposit to ensure appropriate testing
	type MinimumDeposit = ConstU128<ANLOG>;
	type LaunchAdmin = EnsureRoot<AccountId>;
	type WeightInfo = crate::TestWeightInfo;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
