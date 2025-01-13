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
use sp_keyring::{
	AccountKeyring::{Alice, Bob, Charlie},
	Ed25519Keyring::{Dave, Eve, Ferdie},
};
use sp_runtime::{
	traits::Identity, transaction_validity::TransactionLongevity, BuildStorage,
	DispatchError::BadOrigin, TokenError,
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
		claims: vec![(Alice.into(), 100), (Bob.into(), 200), (Dave.into(), 300), (Eve.into(), 400)],
		vesting: vec![(Alice.into(), 50, 10, 1), (Dave.into(), 50, 10, 1)],
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

		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Alice.into()), Some(100));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Bob.into()), Some(200));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Charlie.into()), None);

		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Dave.into()), Some(300));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Eve.into()), Some(400));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Ferdie.into()), None);

		assert_eq!(
			pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Alice.into()),
			Some((50, 10, 1))
		);
		assert_eq!(pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Bob.into()), None);
		assert_eq!(
			pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Dave.into()),
			Some((50, 10, 1))
		);
		assert_eq!(pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Eve.into()), None);
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
fn signer_missmatch_fails() {
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
fn target_missmatch_fails() {
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
fn without_claim_fails() {
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

#[test]
fn mint_claim_works() {
	new_test_ext().execute_with(|| {
		// Non-root are not allowed to add new claims
		assert_noop!(
			Airdrop::mint(RuntimeOrigin::signed(42), Charlie.into(), 200, None),
			sp_runtime::traits::BadOrigin,
		);
		assert_eq!(Balances::free_balance(42), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::HasNoClaim,
		);
		// Root is allowed to add claim
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 200, None));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 200);
		// Minting does not overwrite existing claim
		assert_noop!(
			Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 200, None),
			Error::<Test>::AlreadyHasClaim
		);
		// Added claim can be processed
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&42)[..]).0,
			42,
		));
		assert_eq!(Balances::free_balance(&42), 200);
		assert_eq!(Vesting::vesting_balance(&42), None);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());
	});
}

#[test]
fn mint_claim_with_vesting_works() {
	new_test_ext().execute_with(|| {
		// Non-root user is not able to add claim
		assert_noop!(
			Airdrop::mint(RuntimeOrigin::signed(42), Charlie.into(), 200, Some((50, 10, 1)),),
			sp_runtime::traits::BadOrigin,
		);
		assert_eq!(Balances::free_balance(42), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::HasNoClaim,
		);
		// Root user is able to add claim and vestign is honored
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 200, Some((50, 10, 1)),));
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&42)[..]).0,
			42,
		));
		assert_eq!(Balances::free_balance(&42), 200);
		assert_eq!(Vesting::vesting_balance(&42), Some(50));

		// Make sure we can not transfer the vested balance.
		assert_err!(
			<Balances as Currency<_>>::transfer(&42, &80, 180, ExistenceRequirement::AllowDeath),
			TokenError::Frozen,
		);
	});
}

#[test]
fn origin_signed_claiming_fail() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_err!(
			Airdrop::claim_raw(
				RuntimeOrigin::signed(42),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			sp_runtime::traits::BadOrigin,
		);
	});
}

#[test]
fn double_claiming_doesnt_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(42), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Alice.into(),
			Alice.sign(&Airdrop::to_message(&42)[..]).0,
			42,
		));
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&42)[..]).0,
				42,
			),
			Error::<Test>::HasNoClaim
		);
	});
}

#[test]
fn claiming_while_vested_doesnt_work() {
	new_test_ext().execute_with(|| {
		CurrencyOf::<Test>::make_free_balance_be(&69, total_claims());
		assert_eq!(Balances::free_balance(69), total_claims());
		// A user is already vested
		assert_ok!(<Test as Config>::VestingSchedule::add_vesting_schedule(
			&69,
			total_claims(),
			100,
			10
		));
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 200, Some((50, 10, 1)),));
		// New total
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 200);

		// They should not be able to claim
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&69)[..]).0,
				69
			),
			Error::<Test>::VestedBalanceExists,
		);
	});
}
