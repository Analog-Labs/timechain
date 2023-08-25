use crate::{self as pallet_ocw};
use sp_core::{ConstU128, ConstU16, ConstU32, ConstU64, H256};
use sp_keystore::{testing::MemoryKeystore, Keystore};
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, DispatchResult, MultiSignature,
};
use std::collections::HashMap;
use std::sync::Mutex;
use time_primitives::{
	CycleStatus, Network, OcwShardInterface, OcwTaskInterface, PeerId, PublicKey, ShardId,
	ShardsInterface, TaskCycle, TaskError, TaskId, TasksInterface, TssPublicKey, TIME_KEY_TYPE,
};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

lazy_static::lazy_static! {
	pub static ref SHARD_PUBLIC_KEYS: Mutex<HashMap<ShardId, TssPublicKey>> = Default::default();
}

pub struct MockShards;

impl OcwShardInterface for MockShards {
	fn benchmark_register_shard(
		_network: Network,
		_members: Vec<PeerId>,
		_collector: PublicKey,
		_threshold: u16,
	) {
	}
	fn submit_tss_public_key(shard_id: ShardId, public_key: TssPublicKey) -> DispatchResult {
		SHARD_PUBLIC_KEYS.lock().unwrap().insert(shard_id, public_key);
		Ok(())
	}
	fn set_shard_offline(_shard_id: ShardId) -> DispatchResult {
		Ok(())
	}
}

impl ShardsInterface for MockShards {
	fn is_shard_online(_: ShardId) -> bool {
		true
	}
	fn collector_pubkey(_: ShardId) -> Option<PublicKey> {
		let keystore = MemoryKeystore::new();
		let collector = keystore
			.sr25519_generate_new(TIME_KEY_TYPE, Some(crate::tests::PHRASE))
			.unwrap();
		Some(collector.into())
	}
	fn collector_peer_id(_: ShardId) -> Option<PeerId> {
		Some([0u8; 32])
	}
}

pub struct MockTasks;

impl OcwTaskInterface for MockTasks {
	fn submit_task_result(_: TaskId, _: TaskCycle, _: CycleStatus) -> DispatchResult {
		Ok(())
	}

	fn submit_task_error(_: TaskId, _: TaskError) -> DispatchResult {
		Ok(())
	}

	fn submit_task_hash(_: ShardId, _: TaskId, _: String) -> DispatchResult {
		Ok(())
	}
}

impl TasksInterface for MockTasks {
	fn shard_online(_: ShardId, _: Network) -> DispatchResult {
		Ok(())
	}
	fn shard_offline(_: ShardId, _: Network) -> DispatchResult {
		Ok(())
	}
}

frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Ocw: pallet_ocw::{Pallet, Call, Storage, Event<T>},
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
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

impl pallet_ocw::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type AuthorityId = time_primitives::crypto::SigAuthId;
	type OcwShards = MockShards;
	type OcwTasks = MockTasks;
	type Shards = MockShards;
	type Tasks = MockTasks;
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
