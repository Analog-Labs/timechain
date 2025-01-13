use crate as pallet_airdrop;
use crate::Call;

use super::*;
//use hex_literal::hex;

use scale_codec::Encode;

// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use frame_support::{
	assert_err, assert_noop, assert_ok, derive_impl,
	dispatch::{GetDispatchInfo, Pays},
	ord_parameter_types, parameter_types,
	traits::{ExistenceRequirement, WithdrawReasons},
};
use pallet_balances;
use sp_runtime::{
	traits::Identity, transaction_validity::TransactionLongevity, BuildStorage,
	DispatchError::BadOrigin, TokenError,
};
use sp_keyring::{
	AccountKeyring::{Alice, Bob, Charlie},
	Ed25519Keyring::{Dave, Eve, Ferdie},
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vesting: pallet_vesting,
		Airdrop: pallet_airdrop,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type AccountData = pallet_balances::AccountData<u64>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 1;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = Identity;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub RawPrefix: &'static [u8] = b"Pay RUSTs to the TEST account: ";
}
ord_parameter_types! {
	pub const Six: u64 = 6;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type RawPrefix = RawPrefix;
	type WeightInfo = TestWeightInfo;
}

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// We use default for brevity, but you can configure as desired if needed.
	pallet_balances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_airdrop::GenesisConfig::<Test> {
		claims: vec![
			(Alice.into(), 100),
			(Bob.into(), 200),
			(Dave.into(), 300),
			(Eve.into(), 400),
		],
		vesting: vec![
			(Alice.into(), 50, 10, 1),
			(Dave.into(), 50, 10, 1),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

fn total_claims() -> u64 {
	100 + 200 + 300 + 400
}

#[test]
fn basic_setup_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());

		/*
		assert_eq!(pallet_airdrop::Claims::<Test>::get(Alice.into()), Some(100));
		assert_eq!(pallet_airdrop::Claims::<Test>::get(Bob.into()), Some(200));
		assert_eq!(pallet_airdrop::Claims::<Test>::get(Charlie.into()), None);
		assert_eq!(pallet_airdrop::Claims::<Test>::get(Dave.into()), Some(300));
		assert_eq!(pallet_airdrop::Claims::<Test>::get(Eve.into()), Some(400));
		assert_eq!(pallet_airdrop::Claims::<Test>::get(Ferdie.into()), None);

		assert_eq!(pallet_airdrop::Vesting::<Test>::get(Alice.into()), Some((50, 10, 1)));
		assert_eq!(pallet_airdrop::Vesting::<Test>::get(Bob.into()), None);
		assert_eq!(pallet_airdrop::Vesting::<Test>::get(Charlie.into()), None);
		*/
	});
}

#[test]
fn claim_raw_schnorr_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Alice.into(),
			Alice.sign(&Airdrop::to_message(&42)[..]).0,
			42,
		));
		assert_eq!(Balances::free_balance(&42), 100);
		assert_eq!(Vesting::vesting_balance(&42), Some(50));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 100);
	});
}

#[test]
fn claim_raw_edwards_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Dave.into(),
			Dave.sign(&Airdrop::to_message(&42)[..]).0,
			42,
		));
		assert_eq!(Balances::free_balance(&42), 300);
		assert_eq!(Vesting::vesting_balance(&42), Some(50));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 300);
	});
}

#[test]
fn signer_missmatch_doesnt_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Bob.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::InvalidSignature,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Eve.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::InvalidSignature,
		);

	});
}

#[test]
fn target_missmatch_doesnt_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&69)[..]).0,
				42,
			),
			Error::<Test>::InvalidSignature,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Dave.sign(&Airdrop::to_message(&69)[..]).0,
				42,
			),
			Error::<Test>::InvalidSignature,
		);
	});
}

#[test]
fn non_claimant_doesnt_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::HasNoClaim
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Ferdie.into(),
				Ferdie.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::HasNoClaim
		);
	});
}
