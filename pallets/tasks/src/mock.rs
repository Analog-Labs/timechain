use crate::{self as task_schedule};
use sp_core::{ConstU128, ConstU16, ConstU32, ConstU64, ConstU8, H256};
use sp_runtime::{
	app_crypto::sp_core,
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};
use time_primitives::{Network, PublicKey, ShardId, ShardsInterface, TssPublicKey};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub struct MockShardInterface;

impl ShardsInterface for MockShardInterface {
	fn is_shard_online(_: ShardId) -> bool {
		true
	}

	fn is_shard_member(_: &AccountId) -> bool {
		true
	}

	fn create_shard(_: Network, _: Vec<AccountId>, _: u16) {}

	fn random_signer(shard_id: ShardId) -> PublicKey {
		PublicKey::Sr25519(sp_core::sr25519::Public::from_raw([shard_id as _; 32]))
	}

	fn tss_public_key(_: ShardId) -> Option<TssPublicKey> {
		// this value is taken from running a valid tss cycle
		Some([
			2, 163, 23, 136, 62, 90, 175, 126, 10, 65, 254, 12, 65, 67, 97, 247, 152, 34, 231, 253,
			55, 115, 250, 33, 58, 75, 46, 99, 112, 78, 22, 197, 74,
		])
	}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Tasks: task_schedule::{Pallet, Call, Storage, Event<T>},
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u32;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
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
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

impl task_schedule::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type AuthorityId = time_primitives::crypto::SigAuthId;
	type Shards = MockShardInterface;
	type MaxRetryCount = ConstU8<3>;
	type WritePhaseTimeout = ConstU64<10>;
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
