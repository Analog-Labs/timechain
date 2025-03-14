//! On-chain funding configuration

use polkadot_sdk::*;

use frame_support::{parameter_types, PalletId};

// Local module imports
use crate::{main_or_test, weights, ExistentialDeposit, DefaultAdminOrigin, Runtime, RuntimeEvent, Vesting};

parameter_types! {
	pub RawPrefix: &'static [u8] = main_or_test!(b"Airdrop ANLOG to the Timechain account: ", b"Airdrop TANLOG to the Testnet account: ");
	pub LaunchId: PalletId = PalletId(*b"timelnch");
}

impl pallet_airdrop::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type RawPrefix = RawPrefix;
	type MinimumBalance = ExistentialDeposit;
	type WeightInfo = weights::pallet_airdrop::WeightInfo<Runtime>;
}

impl pallet_launch::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LaunchId;
	type MinimumDeposit = ExistentialDeposit;
	type LaunchAdmin = DefaultAdminOrigin;
	type WeightInfo = weights::pallet_launch::WeightInfo<Runtime>;
}
