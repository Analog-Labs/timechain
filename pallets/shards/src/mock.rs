use crate::{self as pallet_shards};

use polkadot_sdk::{frame_support, frame_system, pallet_balances, sp_core, sp_io, sp_runtime};

use frame_support::derive_impl;
use frame_support::{pallet_prelude::Weight, traits::OnInitialize};
use sp_core::{ConstU128, ConstU64};
use sp_runtime::{
	traits::{IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};
use time_primitives::{ElectionsInterface, NetworkId, ShardId, ShardsInterface, TasksInterface};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub struct MockTasks;

impl TasksInterface for MockTasks {
	fn shard_online(_: ShardId, _: NetworkId) {}
	fn shard_offline(_: ShardId, _: NetworkId) {}
	fn gateway_registered(_: NetworkId, _: u64) {}
}

pub struct MockElections;

impl ElectionsInterface for MockElections {
	fn shard_offline(_: NetworkId, _: Vec<AccountId>) {}
	fn default_shard_size() -> u16 {
		3
	}
	fn member_online(member: &AccountId, network: NetworkId) {
		Shards::member_online(member, network)
	}
	fn member_offline(member: &AccountId, network: NetworkId) -> Weight {
		Shards::member_offline(member, network)
	}
}

frame_support::construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Elections: pallet_elections,
		Members: pallet_members,
		Shards: pallet_shards::{Pallet, Call, Storage, Event<T>},
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
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type Members = Members;
	type Shards = Shards;
}

impl pallet_shards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type Tasks = MockTasks;
	type Members = Members;
	type Elections = Elections;
	type DkgTimeout = ConstU64<10>;
}

impl pallet_members::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Elections = Elections;
	type MinStake = ConstU128<5>;
	type HeartbeatTimeout = ConstU64<10>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		account: AccountId,
		_nonce: u32,
	) -> Option<(
		RuntimeCall,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		Some((call, (account, (), ())))
	}
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Test
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

// roll number of blocks
pub fn roll(n: u64) {
	for _ in 0..n {
		next_block();
	}
}

fn next_block() {
	let mut now = System::block_number();
	now += 1;
	System::set_block_number(now);
	Shards::on_initialize(now);
	Members::on_initialize(now);
	Elections::on_initialize(now);
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(acc_pub(1).into(), 10_000_000_000), (acc_pub(2).into(), 20_000_000_000)],
	}
	.assimilate_storage(&mut storage)
	.unwrap();
	let mut ext: sp_io::TestExternalities = storage.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
}
