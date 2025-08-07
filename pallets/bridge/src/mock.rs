use crate as pallet_bridge;

use polkadot_sdk::*;

use frame_support::{derive_impl, parameter_types, PalletId};
use sp_core::ConstU128;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use time_primitives::{AccountId, Balance};

type Block = frame_system::mocking::MockBlock<Test>;

// Default balance in test accounts
pub const BRIDGING_BALANCE: Balance = 10_000_000_000_000_000;

// Helper to make test accounts
pub fn make_account(id: u8) -> AccountId {
	AccountId::from([id; 32])
}

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Bridge: pallet_bridge,
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
	pub BridgeId: PalletId = PalletId(*b"testbrdg");
}

impl pallet_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = BridgeId;
	type WeightInfo = crate::TestWeightInfo;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(make_account(1), BRIDGING_BALANCE), (make_account(2), BRIDGING_BALANCE)],
		dev_accounts: None,
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	storage.into()
}
