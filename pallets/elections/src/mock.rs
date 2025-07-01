use crate::{self as pallet_elections};

use polkadot_sdk::*;

use frame_support::derive_impl;
use frame_support::traits::OnInitialize;
use sp_core::{ConstU128, ConstU32, ConstU64};
use sp_runtime::{
	traits::{IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};
use time_primitives::{
	Address32, BatchGasParams, NetworkId, NetworksInterface, ShardId, TasksInterface,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub struct MockTasks;

impl TasksInterface for MockTasks {
	fn shard_online(_: ShardId, _: NetworkId) {}
	fn shard_offline(_: ShardId, _: NetworkId) {}
	fn gateway_registered(_: NetworkId, _: u64) {}
	fn network_removed(_network: NetworkId) {}
}

pub struct MockNetworks;

impl NetworksInterface for MockNetworks {
	fn gateway(_network: NetworkId) -> Option<Address32> {
		Some([0; 32])
	}
	fn networks() -> Vec<NetworkId> {
		vec![0]
	}
	fn next_batch_size(_network: NetworkId, _block_height: u64) -> u32 {
		5
	}
	fn batch_gas_params(_network: NetworkId) -> BatchGasParams {
		Default::default()
	}
	fn shard_task_limit(_network: NetworkId) -> u32 {
		10
	}
	fn shard_size(_network: NetworkId) -> u16 {
		3
	}
	fn shard_threshold(_network: NetworkId) -> u16 {
		2
	}
}

frame_support::construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Shards: pallet_shards::{Pallet, Call, Storage, Event<T>},
		Elections: pallet_elections::{Pallet, Storage, Event<T>},
		Members: pallet_members,
		Networks: pallet_networks,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
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

impl pallet_elections::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Shards = Shards;
	type Members = Members;
	type Networks = MockNetworks;
	type MaxElectionsPerBlock = ConstU32<10>;
}

impl pallet_shards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Tasks = MockTasks;
	type Members = Members;
	type Elections = Elections;
	type DkgTimeout = ConstU64<10>;
}

impl pallet_members::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Elections = Elections;
	type Shards = Shards;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type HeartbeatTimeout = ConstU64<10>;
	type MaxTimeoutsPerBlock = ConstU32<100>;
}

impl pallet_networks::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Tasks = MockTasks;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(acc_pub(1).into(), 10_000_000_000),
			(acc_pub(2).into(), 10_000_000_000),
			(acc_pub(3).into(), 10_000_000_000),
			(acc_pub(4).into(), 10_000_000_000),
			(acc_pub(5).into(), 10_000_000_000),
			(acc_pub(6).into(), 10_000_000_000),
			(acc_pub(7).into(), 10_000_000_000),
			(acc_pub(8).into(), 10_000_000_000),
		],
		dev_accounts: None,
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
}

pub fn roll(n: u64) {
	for _ in 0..n {
		next_block();
	}
}

fn next_block() {
	let mut now = System::block_number();
	now += 1;
	System::set_block_number(now);
	Elections::on_initialize(now);
}
