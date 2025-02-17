use crate::{self as pallet_networks};

use polkadot_sdk::frame_support::derive_impl;
use polkadot_sdk::sp_core::{ConstU128, ConstU16, ConstU64};
use polkadot_sdk::sp_runtime::{traits::IdentityLookup, BuildStorage};
use polkadot_sdk::{frame_support, frame_system, pallet_balances, sp_io};
use time_primitives::{NetworkId, ShardId, TasksInterface};

pub struct MockTasks;

impl TasksInterface for MockTasks {
	fn shard_online(_shard_id: ShardId, _network: NetworkId) {}
	fn shard_offline(_shard_id: ShardId, _network: NetworkId) {}
	fn gateway_registered(_network: NetworkId, _block: u64) {}
}

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;
frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Networks: pallet_networks::{Pallet, Call, Storage, Event<T>},
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

impl pallet_networks::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Tasks = MockTasks;
	type TimechainNetworkId = ConstU16<1000>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(1_u64, 10_000_000_000)],
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
