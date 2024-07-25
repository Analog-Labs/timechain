use crate::{self as task_schedule};
use core::marker::PhantomData;

use polkadot_sdk::{
	frame_support, frame_system, pallet_balances, pallet_treasury, sp_core, sp_io, sp_runtime,
	sp_std,
};

use frame_support::derive_impl;
use frame_support::traits::{
	tokens::{ConversionFromAssetBalance, Pay, PaymentStatus},
	OnFinalize, OnInitialize,
};
use frame_support::PalletId;

use sp_core::{ConstU128, ConstU32, ConstU64};
use sp_runtime::{
	traits::{parameter_types, Get, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, DispatchResult, MultiSignature, Percent, Permill,
};
use sp_std::cell::RefCell;
use sp_std::collections::btree_map::BTreeMap;

use schnorr_evm::SigningKey;

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
		Members: pallet_members,
		Elections: pallet_elections,
		Treasury: pallet_treasury,
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

thread_local! {
	pub static PAID: RefCell<BTreeMap<(u128, u32), u128>> = const { RefCell::new(BTreeMap::new()) };
	pub static STATUS: RefCell<BTreeMap<u64, PaymentStatus>> = const { RefCell::new(BTreeMap::new()) };
	pub static LAST_ID: RefCell<u64> = const { RefCell::new(0u64) };
}

/// set status for a given payment id
fn set_status(id: u64, s: PaymentStatus) {
	STATUS.with(|m| m.borrow_mut().insert(id, s));
}

pub struct TestPay;
impl Pay for TestPay {
	type Beneficiary = u128;
	type Balance = u128;
	type Id = u64;
	type AssetKind = u32;
	type Error = ();

	fn pay(
		who: &Self::Beneficiary,
		asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		PAID.with(|paid| *paid.borrow_mut().entry((*who, asset_kind)).or_default() += amount);
		Ok(LAST_ID.with(|lid| {
			let x = *lid.borrow();
			lid.replace(x + 1);
			x
		}))
	}
	fn check_payment(id: Self::Id) -> PaymentStatus {
		STATUS.with(|s| s.borrow().get(&id).cloned().unwrap_or(PaymentStatus::Unknown))
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_: &Self::Beneficiary, _: Self::AssetKind, _: Self::Balance) {}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_concluded(id: Self::Id) {
		set_status(id, PaymentStatus::Failure)
	}
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const Burn: Permill = Permill::from_percent(50);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub TreasuryAccount: AccountId = Treasury::account_id();
	pub const SpendPayoutPeriod: u64 = 5;
}
pub struct TestSpendOrigin;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for TestSpendOrigin {
	type Success = u128;
	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Result::<frame_system::RawOrigin<_>, RuntimeOrigin>::from(o).and_then(|o| match o {
			frame_system::RawOrigin::Root => Ok(u128::MAX),
			r => Err(RuntimeOrigin::from(r)),
		})
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

pub struct MulBy<N>(PhantomData<N>);
impl<N: Get<u128>> ConversionFromAssetBalance<u128, u32, u128> for MulBy<N> {
	type Error = ();
	fn from_asset_balance(balance: u128, _asset_id: u32) -> Result<u128, Self::Error> {
		balance.checked_mul(N::get()).ok_or(())
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_: u32) {}
}

impl pallet_treasury::Config for Test {
	type PalletId = TreasuryPalletId;
	type Currency = pallet_balances::Pallet<Test>;
	type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ConstU128<1>;
	type ProposalBondMaximum = ();
	type SpendPeriod = ConstU64<2>;
	type Burn = Burn;
	type BurnDestination = (); // Just gets burned.
	type WeightInfo = ();
	type SpendFunds = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = TestSpendOrigin;
	type AssetKind = u32;
	type Beneficiary = u128;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = TestPay;
	type BalanceConverter = MulBy<ConstU128<2>>;
	type PayoutPeriod = SpendPayoutPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_members::Config for Test {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Elections = Shards;
	type MinStake = ConstU128<5>;
	type HeartbeatTimeout = ConstU64<10>;
}

impl pallet_elections::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type Shards = Shards;
	type Members = Members;
}

parameter_types! {
	pub const PalletIdentifier: PalletId = PalletId(*b"py/tasks");
	// reward declines by 5% every 10 blocks
	pub const RewardDeclineRate: DepreciationRate<u64> = DepreciationRate { blocks: 2, percent: Percent::from_percent(50) };
}

impl pallet_shards::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type TaskScheduler = Tasks;
	type Members = MockMembers;
	type Elections = Elections;
	type DkgTimeout = ConstU64<10>;
}

impl task_schedule::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AdminOrigin = frame_system::EnsureRoot<AccountId>;
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
		balances: vec![
			(acc_pub(0).into(), 10_000_000_000),
			(acc_pub(1).into(), 20_000_000_000),
			(TreasuryAccount::get(), 30_000_000_000),
		],
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

pub fn roll(n: u64) {
	for _ in 0..n {
		next_block();
	}
}

fn next_block() {
	let mut now = System::block_number();
	Tasks::on_finalize(now);
	now += 1;
	System::set_block_number(now);
	Shards::on_initialize(now);
	Tasks::on_initialize(now);
}
