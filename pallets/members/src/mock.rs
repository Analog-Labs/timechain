use crate::{self as pallet_members};
use frame_support::traits::OnInitialize;
use sp_core::{ConstU128, ConstU16, ConstU32, ConstU64, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};
use time_primitives::{MemberEvents, NetworkId};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub struct MockShards;

impl MemberEvents for MockShards {
	fn member_online(_: &AccountId, _: NetworkId) {}
	fn member_offline(_: &AccountId, _: NetworkId) {}
}

frame_support::construct_runtime!(
	pub struct Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Members: pallet_members::{Pallet, Call, Storage, Event<T>},
	}
);

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

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type RuntimeTask = RuntimeTask;
	type Nonce = u32;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u128;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
}

impl pallet_members::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Elections = MockShards;
	type Currency = Balances;
	type MinStake = ConstU128<5>;
	type HeartbeatTimeout = ConstU64<10>;
}

/// To from `now` to block `n`.
pub fn roll_to(n: u64) {
	let now = System::block_number();
	for i in now + 1..=n {
		System::set_block_number(i);
		Members::on_initialize(i);
	}
}

fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
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
