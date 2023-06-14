use crate::{self as pallet_tesseract_sig_storage};
use frame_support::{
	pallet_prelude::*,
	parameter_types,
	traits::{ConstU16, ConstU64, OnTimestampSet, ValidatorSet},
	PalletId,
};
use frame_system as system;
use sp_core::{ConstU32, H256};
use sp_runtime::MultiSignature;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, Convert, IdentifyAccount, IdentityLookup, Verify},
	Permill,
};

use sp_std::cell::RefCell;
use time_primitives::TimeId;
// use pallet_randomness_collective_flip;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
/// Type used for expressing timestamp.
type Moment = u64;
pub type Signature = MultiSignature;

/// An index to a block.
pub type BlockNumber = u32;
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

pub const MILLICENTS: Balance = 1_000_000_000;
pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
pub const DOLLARS: Balance = 100 * CENTS;
type Balance = u128;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Index = u64;

pub const VALIDATOR_1: AccountId = AccountId::new([1; 32]);
pub const VALIDATOR_2: AccountId = AccountId::new([2; 32]);
pub const VALIDATOR_3: AccountId = AccountId::new([3; 32]);
pub const VALIDATOR_4: AccountId = AccountId::new([4; 32]);

pub const INVALID_VALIDATOR: AccountId = AccountId::new([100; 32]);

pub const ALICE: TimeId = TimeId::new([1u8; 32]);
pub const BOB: TimeId = TimeId::new([2u8; 32]);
pub const CHARLIE: TimeId = TimeId::new([3u8; 32]);
pub const DJANGO: TimeId = TimeId::new([4u8; 32]);

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		Timestamp: pallet_timestamp,
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		PalletProxy: pallet_proxy::{Pallet, Call, Storage, Event<T>},
		Treasury: pallet_treasury::{Pallet, Call, Storage, Event<T>},
		TaskSchedule: task_schedule::{Pallet, Call, Storage, Event<T>},
		TesseractSigStorage: pallet_tesseract_sig_storage::{Pallet, Call, Storage, Event<T>},
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = Index;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
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

thread_local! {
	pub static CAPTURED_MOMENT: RefCell<Option<Moment>> = RefCell::new(None);
}

parameter_types! {
	// Must be > 0 and <= 100
	pub const SlashingPercentage: u8 = 5;
	// Must be > 0 and <= 100
	pub const SlashingPercentageThreshold: u8 = 51;
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = DOLLARS;
	pub const SpendPeriod: BlockNumber = DAYS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = Balance::max_value();
	pub static Burn: Permill = Permill::from_percent(50);
	pub const ExistentialDeposit: u64 = 1;
	pub const BlockHashCount: BlockNumber = 2400;
	pub const ScheduleFee: Balance = 1;
}
pub struct MockOnTimestampSet;
impl OnTimestampSet<Moment> for MockOnTimestampSet {
	fn on_timestamp_set(moment: Moment) {
		CAPTURED_MOMENT.with(|x| *x.borrow_mut() = Some(moment));
	}
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ();
	type MaxFreezes = ();
}

impl pallet_treasury::Config for Test {
	type PalletId = TreasuryPalletId;
	type Currency = pallet_balances::Pallet<Test>;
	type ApproveOrigin = frame_system::EnsureRoot<AccountId>;
	type RejectOrigin = frame_system::EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ();
	type SpendPeriod = ConstU64<2>;
	type Burn = Burn;
	type BurnDestination = (); // Just gets burned.
	type WeightInfo = ();
	type SpendFunds = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;
}

pub struct CurrentPalletAccounts;
impl time_primitives::PalletAccounts<AccountId> for CurrentPalletAccounts {
	fn get_treasury() -> AccountId {
		Treasury::account_id()
	}
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = MockOnTimestampSet;
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl pallet_proxy::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_proxy::weights::WeightInfo<Test>;
	type Currency = ();
}

impl task_schedule::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = task_schedule::weights::WeightInfo<Test>;
	type ProxyExtend = ();
	type Currency = Balances;
	type PalletAccounts = CurrentPalletAccounts;
	type ScheduleFee = ScheduleFee;
	type AuthorityId = task_schedule::crypto::SigAuthId;
}

pub struct ConvertMock<T>(sp_std::marker::PhantomData<T>);

impl<AccountId> Convert<AccountId, Option<AccountId>> for ConvertMock<AccountId> {
	fn convert(account_id: AccountId) -> Option<AccountId> {
		Some(account_id)
	}
}

pub struct ValidatorSetMock<T>(sp_std::marker::PhantomData<T>);

impl<OutAccountId> ValidatorSet<OutAccountId> for ValidatorSetMock<OutAccountId>
where
	OutAccountId: Clone
		+ Eq
		+ PartialEq
		+ Parameter
		+ MaxEncodedLen
		+ From<AccountId>
		+ std::convert::From<sp_runtime::AccountId32>,
{
	type ValidatorId = OutAccountId;
	type ValidatorIdOf = ConvertMock<OutAccountId>;

	fn session_index() -> sp_staking::SessionIndex {
		sp_staking::SessionIndex::default()
	}

	fn validators() -> Vec<Self::ValidatorId> {
		vec![VALIDATOR_1.into(), VALIDATOR_2.into(), VALIDATOR_3.into(), VALIDATOR_4.into()]
	}
}

impl pallet_tesseract_sig_storage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Moment = Moment;
	type Timestamp = Timestamp;
	type SlashingPercentage = SlashingPercentage;
	type SlashingPercentageThreshold = SlashingPercentageThreshold;
	type TaskScheduleHelper = TaskSchedule;
	type ValidatorSet = ValidatorSetMock<AccountId>;
	type MaxChronicleWorkers = ConstU32<3>;
	type AuthorityId = pallet_tesseract_sig_storage::crypto::SigAuthId;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		account: AccountId,
		_nonce: Index,
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
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
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
