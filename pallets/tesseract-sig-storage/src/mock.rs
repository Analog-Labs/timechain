use crate as pallet_tesseract_sig_storage;
use frame_support::traits::{ConstU16, ConstU64, OnTimestampSet};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use sp_std::cell::RefCell;

// use pallet_randomness_collective_flip;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
/// Type used for expressing timestamp.
type Moment = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Timestamp: pallet_timestamp,
		System: frame_system,
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
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
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

pub struct MockOnTimestampSet;
impl OnTimestampSet<Moment> for MockOnTimestampSet {
	fn on_timestamp_set(moment: Moment) {
		CAPTURED_MOMENT.with(|x| *x.borrow_mut() = Some(moment));
	}
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = MockOnTimestampSet;
	type MinimumPeriod = ConstU64<5>;
	type WeightInfo = ();
}

impl pallet_tesseract_sig_storage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type StoreRandomness = RandomnessCollectiveFlip;
	type Moment = Moment;
	type Timestamp = Timestamp;
}

impl pallet_randomness_collective_flip::Config for Test {}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext: sp_io::TestExternalities =
		GenesisConfig::default().build_storage().unwrap().into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
