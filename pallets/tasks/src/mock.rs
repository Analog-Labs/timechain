use crate::{self as task_schedule};
use frame_support::derive_impl;
use frame_support::traits::OnInitialize;
use frame_support::PalletId;
use schnorr_evm::SigningKey;
use sp_core::{ConstU128, ConstU64};
use sp_runtime::{
	traits::{parameter_types, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, DispatchResult, MultiSignature, Percent,
};
use time_primitives::{
	Balance, DepreciationRate, ElectionsInterface, MemberStorage, NetworkId, PeerId, PublicKey,
	TransferStake,
};

pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub struct MockMembers;

impl MemberStorage for MockMembers {
	fn member_stake(_: &AccountId) -> Balance {
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
	fn total_stake() -> u128 {
		0u128
	}
}
impl TransferStake for MockMembers {
	fn transfer_stake(_: &AccountId, _: &AccountId, _: Balance) -> DispatchResult {
		Ok(())
	}
}

pub struct MockElections;

impl ElectionsInterface for MockElections {
	fn shard_offline(_: NetworkId, _: Vec<AccountId>) {}
	fn default_shard_size() -> u16 {
		3
	}
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

parameter_types! {
	pub const PalletIdentifier: PalletId = PalletId(*b"py/tasks");
	// reward declines by 5% every 10 blocks
	pub const RewardDeclineRate: DepreciationRate<u64> = DepreciationRate { blocks: 2, percent: Percent::from_percent(50) };
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
	type Elections = MockElections;
	type Shards = Shards;
	type Members = MockMembers;
	type BaseReadReward = ConstU128<2>;
	type BaseWriteReward = ConstU128<3>;
	type BaseSendMessageReward = ConstU128<4>;
	type RewardDeclineRate = RewardDeclineRate;
	type SignPhaseTimeout = ConstU64<10>;
	type WritePhaseTimeout = ConstU64<10>;
	type ReadPhaseTimeout = ConstU64<20>;
	type PalletId = PalletIdentifier;
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

	pub fn sign(&self, data: &[u8]) -> schnorr_evm::Signature {
		self.signing_key.sign(data)
	}
}

/// To from `now` to block `n`.
pub fn roll_to(n: u64) {
	let now = System::block_number();
	for i in now + 1..=n {
		System::set_block_number(i);
		Shards::on_initialize(i);
		Tasks::on_initialize(i);
	}
}
