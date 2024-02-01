use crate::{self as task_schedule};
use frame_support::traits::OnInitialize;
use frame_support::PalletId;
use schnorr_evm::{SigningKey, VerifyingKey};
use sp_core::{ConstU128, ConstU16, ConstU32, ConstU64, ConstU8, H256};
use sp_runtime::{
	app_crypto::sp_core,
	traits::{parameter_types, BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature, Percent,
};
use sp_std::vec::Vec;
use time_primitives::{
	append_hash_with_task_data, DepreciationRate, ElectionsInterface, Function, MemberStorage,
	Network, PeerId, PublicKey, RewardConfig, ShardId, ShardStatus, ShardsInterface, TaskCycle,
	TaskDescriptor, TaskDescriptorParams, TaskError, TaskExecution, TaskId, TaskPhase, TaskResult,
	TaskStatus, TasksInterface,
};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub struct MockMembers;

impl MemberStorage for MockMembers {
	type Balance = u128;
	fn member_stake(_: &AccountId) -> Self::Balance {
		0u128
	}
	fn member_peer_id(_: &AccountId) -> Option<PeerId> {
		None
	}
	fn member_public_key(_account: &AccountId) -> Option<PublicKey> {
		Some(sp_runtime::MultiSigner::Sr25519(sp_core::sr25519::Public::from_raw([0u8; 32])))
	}
	fn is_member_online(_: &AccountId) -> bool {
		true
	}
}

pub struct MockElections;

impl ElectionsInterface for MockElections {
	fn shard_offline(_: Network, _: Vec<AccountId>) {}
}

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Tasks: task_schedule::{Pallet, Call, Storage, Event<T>},
		Shards: pallet_shards::{Pallet, Call, Storage, Event<T>},
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

parameter_types! {
	pub const PalletIdentifier: PalletId = PalletId(*b"py/tasks");
	// reward declines by 5% every 10 blocks
	pub const RewardDeclineRate: DepreciationRate<u64> = DepreciationRate { blocks: 10, percent: Percent::from_percent(5) };
}

impl pallet_shards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type TaskScheduler = Tasks;
	type Members = MockMembers;
	type Elections = MockElections;
	type DkgTimeout = ConstU64<10>;
}

impl task_schedule::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Shards = Shards;
	type MaxRetryCount = ConstU8<3>;
	type MinTaskBalance = ConstU128<10>;
	type BaseReadReward = ConstU128<2>;
	type BaseWriteReward = ConstU128<3>;
	type BaseSendMessageReward = ConstU128<4>;
	type RewardDeclineRate = RewardDeclineRate;
	type WritePhaseTimeout = ConstU64<10>;
	type ReadPhaseTimeout = ConstU64<20>;
	type PalletId = PalletIdentifier;
}

/// To from `now` to block `n`.
pub fn roll_to(n: u64) {
	let now = System::block_number();
	for i in now + 1..=n {
		System::set_block_number(i);
		Tasks::on_initialize(i);
	}
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
		balances: vec![(acc_pub(0).into(), 10_000_000_000), (acc_pub(1).into(), 20_000_000_000)],
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

pub struct MockTssSigner {
	signing_key: SigningKey,
}

impl MockTssSigner {
	pub fn new() -> Self {
		Self {
			//random key bytes
			signing_key: SigningKey::from_bytes([
				62, 78, 161, 128, 140, 236, 177, 67, 143, 75, 171, 207, 104, 60, 36, 95, 104, 71,
				17, 91, 237, 184, 132, 165, 52, 240, 194, 4, 138, 196, 89, 176,
			])
			.unwrap(),
		}
	}

	pub fn public_key(&self) -> [u8; 33] {
		self.signing_key.public().to_bytes().unwrap()
	}

	pub fn sign(&self, data: [u8; 32]) -> schnorr_evm::Signature {
		self.signing_key.sign_prehashed(data)
	}
}

pub fn shard_size_2() -> [AccountId; 2] {
	[[1u8; 32].into(), [2u8; 32].into()]
}

pub fn shard_size_3() -> [AccountId; 3] {
	[[0u8; 32].into(), [1u8; 32].into(), [2u8; 32].into()]
}

pub fn pubkey_from_bytes(bytes: [u8; 32]) -> PublicKey {
	PublicKey::Sr25519(sp_core::sr25519::Public::from_raw(bytes))
}

pub const A: [u8; 32] = [1u8; 32];

pub fn mock_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		cycle,
		start: 0,
		period: 1,
		timegraph: None,
		function: Function::EvmViewCall {
			address: Default::default(),
			input: Default::default(),
		},
		funds: 100,
		shard_size: 3,
	}
}

pub fn mock_sign_task(network: Network, cycle: TaskCycle) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		cycle,
		start: 0,
		period: 1,
		timegraph: None,
		function: Function::SendMessage {
			address: Default::default(),
			gas_limit: Default::default(),
			salt: Default::default(),
			payload: Default::default(),
		},
		funds: 100,
		shard_size: 3,
	}
}

pub fn mock_payable(network: Network) -> TaskDescriptorParams {
	TaskDescriptorParams {
		network,
		cycle: 1,
		start: 0,
		period: 0,
		timegraph: None,
		function: Function::EvmCall {
			address: Default::default(),
			input: Default::default(),
			amount: 0,
		},
		funds: 100,
		shard_size: 3,
	}
}

pub fn mock_result_ok(shard_id: ShardId, task_id: TaskId, task_cycle: TaskCycle) -> TaskResult {
	// these values are taken after running a valid instance of submitting result
	let hash = [
		11, 210, 118, 190, 192, 58, 251, 12, 81, 99, 159, 107, 191, 242, 96, 233, 203, 127, 91, 0,
		219, 14, 241, 19, 45, 124, 246, 145, 176, 169, 138, 11,
	];
	let appended_hash = append_hash_with_task_data(hash, task_id, task_cycle);
	let final_hash = VerifyingKey::message_hash(&appended_hash);
	let signature = MockTssSigner::new().sign(final_hash).to_bytes();
	TaskResult { shard_id, hash, signature }
}

pub fn mock_error_result(shard_id: ShardId, task_id: TaskId, task_cycle: TaskCycle) -> TaskError {
	// these values are taken after running a valid instance of submitting error
	let msg: String = "Invalid input length".into();
	let msg_hash = VerifyingKey::message_hash(&msg.clone().into_bytes());
	let hash = append_hash_with_task_data(msg_hash, task_id, task_cycle);
	let final_hash = VerifyingKey::message_hash(&hash);
	let signature = MockTssSigner::new().sign(final_hash).to_bytes();
	TaskError { shard_id, msg, signature }
}
