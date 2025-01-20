use crate as pallet_airdrop;
use crate::Call as AirdropCall;

use super::*;
//use hex_literal::hex;

use scale_codec::Encode;

// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use frame_support::{
	assert_err, assert_noop, assert_ok, derive_impl, ord_parameter_types, parameter_types,
	traits::{ExistenceRequirement, WithdrawReasons},
};
use sp_core::ConstU64;
use sp_keyring::{
	AccountKeyring::{Alice, Bob, Charlie},
	Ed25519Keyring::{Dave, Eve, Ferdie},
};
use sp_runtime::{
	traits::{Identity, IdentityLookup},
	transaction_validity::TransactionLongevity,
	BuildStorage, TokenError,
};

use time_primitives::AccountId;

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
	type AccountId = AccountId;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
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
	const MAX_VESTING_SCHEDULES: u32 = 1;
}

parameter_types! {
	// Currently needs to match the prefix to be benchmarked with as we can not sign in wasm.
	pub RawPrefix: &'static [u8] = b"Airdrop TANLOG to the Testnet account: ";
}

ord_parameter_types! {
	pub const Six: u64 = 6;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type VestingSchedule = Vesting;
	type RawPrefix = RawPrefix;
	type MinimumBalance = ConstU64<500>;
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
			(Alice.into(), 1000),
			(Bob.into(), 2000),
			(Dave.into(), 3000),
			(Eve.into(), 4000),
		],
		vesting: vec![(Alice.into(), 50, 10, 1), (Dave.into(), 50, 10, 1)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

fn total_claims() -> u64 {
	1000 + 2000 + 3000 + 4000
}

#[test]
fn basic_setup_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());

		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Alice.into()), Some(1000));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Bob.into()), Some(2000));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Charlie.into()), None);

		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Dave.into()), Some(3000));
		assert_eq!(pallet_airdrop::Claims::<Test>::get::<AccountId32>(Eve.into()), Some(4000));
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
		let alice = Alice.into();
		assert_eq!(Balances::free_balance(&alice), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Alice.into(),
			Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&alice), 1000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), Some(50));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 1000);
	});
}

#[test]
fn claim_raw_edwards_works() {
	new_test_ext().execute_with(|| {
		let alice = Alice.into();
		assert_eq!(Balances::free_balance(&alice), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Dave.into(),
			Dave.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&alice), 3000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), Some(50));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 3000);
	});
}

#[test]
fn signer_missmatch_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Bob.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::InvalidSignature,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Eve.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::InvalidSignature,
		);
	});
}

#[test]
fn target_missmatch_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&Bob.into())[..]).0,
				Charlie.into(),
			),
			Error::<Test>::InvalidSignature,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Dave.sign(&Airdrop::to_message(&Eve.into())[..]).0,
				Ferdie.into(),
			),
			Error::<Test>::InvalidSignature,
		);
	});
}

#[test]
fn without_claim_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Ferdie.into(),
				Ferdie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim
		);
	});
}

#[test]
fn mint_works() {
	new_test_ext().execute_with(|| {
		let alice = Alice.into();
		// Non-root are not allowed to add new claims
		assert_noop!(
			Airdrop::mint(RuntimeOrigin::signed(Alice.into()), Charlie.into(), 1000, None),
			sp_runtime::traits::BadOrigin,
		);
		assert_eq!(Balances::free_balance(&alice), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim,
		);
		// Root is allowed to add claim
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 1000, None));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 1000);
		// Minting does not overwrite existing claim
		assert_noop!(
			Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 1000, None),
			Error::<Test>::AlreadyHasClaim
		);
		// Added claim can be processed
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&alice), 1000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), None);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());
	});
}

#[test]
fn mint_with_vesting_works() {
	new_test_ext().execute_with(|| {
		let alice = Alice.into();
		// Non-root user is not able to add claim
		assert_noop!(
			Airdrop::mint(
				RuntimeOrigin::signed(Alice.into()),
				Charlie.into(),
				1000,
				Some((50, 10, 1)),
			),
			sp_runtime::traits::BadOrigin,
		);
		assert_eq!(Balances::free_balance(&alice), 0);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim,
		);
		// Root user is able to add claim and vestign is honored
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 500, Some((500, 10, 1)),));
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&alice), 500);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), Some(500));

		// Make sure we can not transfer the vested balance.
		assert_err!(
			<Balances as Currency<_>>::transfer(
				&Alice.into(),
				&Bob.into(),
				480,
				ExistenceRequirement::AllowDeath
			),
			TokenError::Frozen,
		);
	});
}

#[test]
fn origin_signed_claiming_fail() {
	new_test_ext().execute_with(|| {
		assert_err!(
			Airdrop::claim_raw(
				RuntimeOrigin::signed(Alice.into()),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			sp_runtime::traits::BadOrigin,
		);
	});
}

#[test]
fn double_claiming_fails() {
	new_test_ext().execute_with(|| {
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Alice.into(),
			Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim
		);
	});
}

#[test]
fn claims_exceeding_vesting_fails() {
	new_test_ext().execute_with(|| {
		let charlie = Charlie.into();
		CurrencyOf::<Test>::make_free_balance_be(&Charlie.into(), total_claims());
		assert_eq!(Balances::free_balance(&charlie), total_claims());
		// A user is already vested and the vesting limit is one
		assert_ok!(<Test as Config>::VestingSchedule::add_vesting_schedule(
			&Charlie.into(),
			total_claims(),
			100,
			10
		));
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 1000, Some((500, 10, 1)),));
		// New total
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 1000);

		// They should not be able to claim
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&Charlie.into())[..]).0,
				Charlie.into()
			),
			Error::<Test>::VestingNotPossible,
		);
	});
}

#[test]
fn validate_unsigned_works() {
	use sp_runtime::traits::ValidateUnsigned;
	let source = sp_runtime::transaction_validity::TransactionSource::External;

	new_test_ext().execute_with(|| {
		// Allow transaction with a valid schnorr signature
		assert_eq!(
			Pallet::<Test>::validate_unsigned(
				source,
				&AirdropCall::claim_raw {
					source: Alice.into(),
					proof: Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
					target: Alice.into(),
				}
			),
			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides: vec![("airdrop", AccountId32::from(Alice)).encode()],
				longevity: TransactionLongevity::MAX,
				propagate: true,
			})
		);
		// Allow transaction with a valid edwards signature
		assert_eq!(
			Pallet::<Test>::validate_unsigned(
				source,
				&AirdropCall::claim_raw {
					source: Dave.into(),
					proof: Dave.sign(&Airdrop::to_message(&Dave.into())[..]).0,
					target: Dave.into(),
				}
			),
			Ok(ValidTransaction {
				priority: 100,
				requires: vec![],
				provides: vec![("airdrop", AccountId32::from(Dave)).encode()],
				longevity: TransactionLongevity::MAX,
				propagate: true,
			})
		);
		// Fail transaction with an invalid proof
		assert_eq!(
			Pallet::<Test>::validate_unsigned(
				source,
				&AirdropCall::claim_raw {
					source: Alice.into(),
					proof: [0u8; 64],
					target: Alice.into(),
				}
			),
			InvalidTransaction::BadProof.into(),
		);
		// Fail transaction without any claim behind them
		assert_eq!(
			Pallet::<Test>::validate_unsigned(
				source,
				&AirdropCall::claim_raw {
					source: Charlie.into(),
					proof: Charlie.sign(&Airdrop::to_message(&Charlie.into())[..]).0,
					target: Charlie.into(),
				}
			),
			InvalidTransaction::BadSigner.into(),
		);
	});
}
