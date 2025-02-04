#![allow(clippy::needless_borrows_for_generic_args)]

use crate as pallet_airdrop;
use crate::Call as AirdropCall;

use super::*;
use mock::{
	new_test_ext, total_claims, Airdrop, Alice, Balances, Bob, Charlie, Dave, Eve, Ferdie,
	RuntimeOrigin, Test, Vesting,
};

use scale_codec::Encode;

use frame_support::{assert_err, assert_noop, assert_ok, traits::ExistenceRequirement};
use sp_runtime::{transaction_validity::TransactionLongevity, TokenError};

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
			Some((800, 10, 1))
		);
		assert_eq!(pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Bob.into()), None);
		assert_eq!(
			pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Dave.into()),
			Some((2400, 10, 1))
		);
		assert_eq!(pallet_airdrop::Vesting::<Test>::get::<AccountId32>(Eve.into()), None);
	});
}

#[test]
fn claim_raw_schnorr_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&Alice.into()), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Alice.into(),
			Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&Alice.into()), 1000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), Some(800));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 1000);

		assert_eq!(Balances::free_balance(&Charlie.into()), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Bob.into(),
			Bob.sign(&Airdrop::to_message(&Charlie.into())[..]).0,
			Charlie.into(),
		));
		assert_eq!(Balances::free_balance(&Charlie.into()), 2000);
		assert_eq!(Vesting::vesting_balance(&Charlie.into()), None);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 3000);
	});
}

#[test]
fn invalid_signature_schnorr_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(RuntimeOrigin::none(), Alice.into(), [0u8; 64], Alice.into(),),
			Error::<Test>::InvalidSignature,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Bob.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::InvalidSignature,
		);
	});
}

#[test]
fn target_missmatch_schnorr_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&Bob.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::InvalidSignature,
		);
	});
}

#[test]
fn claim_raw_edwards_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(&Dave.into()), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Dave.into(),
			Dave.sign(&Airdrop::to_message(&Dave.into())[..]).0,
			Dave.into(),
		));
		assert_eq!(Balances::free_balance(&Dave.into()), 3000);
		assert_eq!(Vesting::vesting_balance(&Dave.into()), Some(2400));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 3000);

		assert_eq!(Balances::free_balance(&Ferdie.into()), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Eve.into(),
			Eve.sign(&Airdrop::to_message(&Ferdie.into())[..]).0,
			Ferdie.into(),
		));
		assert_eq!(Balances::free_balance(&Ferdie.into()), 4000);
		assert_eq!(Vesting::vesting_balance(&Ferdie.into()), None);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() - 7000);
	});
}

#[test]
fn invalid_signature_edwards_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(RuntimeOrigin::none(), Dave.into(), [0u8; 64], Dave.into(),),
			Error::<Test>::InvalidSignature,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Eve.sign(&Airdrop::to_message(&Dave.into())[..]).0,
				Dave.into(),
			),
			Error::<Test>::InvalidSignature,
		);
	});
}

#[test]
fn target_missmatch_edwards_fails() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Dave.sign(&Airdrop::to_message(&Eve.into())[..]).0,
				Dave.into(),
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
				Charlie.sign(&Airdrop::to_message(&Charlie.into())[..]).0,
				Charlie.into(),
			),
			Error::<Test>::HasNoClaim
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Ferdie.into(),
				Ferdie.sign(&Airdrop::to_message(&Ferdie.into())[..]).0,
				Ferdie.into(),
			),
			Error::<Test>::HasNoClaim
		);
	});
}

#[test]
fn mint_works() {
	new_test_ext().execute_with(|| {
		// Non-root are not allowed to add new claims
		assert_eq!(Balances::free_balance(&Alice.into()), 0);
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
		assert_eq!(Balances::free_balance(&Alice.into()), 1000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), None);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());
	});
}

#[test]
fn add_works() {
	new_test_ext().execute_with(|| {
		// Non-root are not allowed to add new claims
		assert_eq!(Balances::free_balance(&Alice.into()), 0);
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
		assert_ok!(Airdrop::add_airdrop(Charlie.into(), 1000, None));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 1000);
		// Add combines with existing claim
		assert_ok!(Airdrop::add_airdrop(Charlie.into(), 1000, None));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 2000);
		// Added claim can be processed
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&Alice.into()), 2000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), None);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());
	});
}

#[test]
fn add_vested_works() {
	new_test_ext().execute_with(|| {
		// Non-root are not allowed to add new claims
		assert_eq!(Balances::free_balance(&Alice.into()), 0);
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
		assert_ok!(Airdrop::add_airdrop(Charlie.into(), 1000, None));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 1000);
		// Additional claims can be added
		assert_ok!(Airdrop::add_airdrop(Charlie.into(), 1000, None));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 2000);
		// Vesting can be added once
		assert_ok!(Airdrop::add_airdrop(Charlie.into(), 1000, Some((800, 80, 10))));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 3000);
		// Second vesting does not overwrite existing claim
		assert_noop!(
			Airdrop::add_airdrop(Charlie.into(), 1000, Some((800, 80, 10))),
			Error::<Test>::VestingNotPossible
		);
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims() + 3000);
		// Added claim can be processed
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&Alice.into()), 3000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), Some(800));
		assert_eq!(pallet_airdrop::Total::<Test>::get(), total_claims());
	});
}

#[test]
fn rootless_mint_move_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::mint(RuntimeOrigin::signed(Alice.into()), Charlie.into(), 1000, None),
			sp_runtime::traits::BadOrigin,
		);

		assert_noop!(
			Airdrop::mint(
				RuntimeOrigin::signed(Bob.into()),
				Charlie.into(),
				1000,
				Some((800, 10, 1)),
			),
			sp_runtime::traits::BadOrigin,
		);
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim,
		);

		assert_noop!(
			Airdrop::transfer(RuntimeOrigin::signed(Alice.into()), Alice.into(), Charlie.into()),
			sp_runtime::traits::BadOrigin,
		);

		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Charlie.into(),
				Charlie.sign(&Airdrop::to_message(&Charlie.into())[..]).0,
				Charlie.into(),
			),
			Error::<Test>::HasNoClaim,
		);
	})
}

#[test]
fn mint_with_vesting_works() {
	new_test_ext().execute_with(|| {
		// Root user is able to add claim and vestign is honored
		assert_ok!(Airdrop::mint(RuntimeOrigin::root(), Charlie.into(), 500, Some((500, 10, 1)),));
		assert_eq!(Balances::free_balance(&Alice.into()), 0);
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&Alice.into()), 500);
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
fn move_with_vesting_works() {
	new_test_ext().execute_with(|| {
		// Root user can not use claim that does not exist
		assert_noop!(
			Airdrop::transfer(RuntimeOrigin::root(), Charlie.into(), Ferdie.into()),
			Error::<Test>::HasNoClaim
		);
		// Root user is able to move claim and vesting is honored
		assert_ok!(Airdrop::transfer(RuntimeOrigin::root(), Alice.into(), Charlie.into()));

		// Alice transferred her airdrop to Charlie
		assert_eq!(Balances::free_balance(&Alice.into()), 0);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), None);

		// Charlie claims it to be paid out back to Alice
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Charlie.into(),
			Charlie.sign(&Airdrop::to_message(&Alice.into())[..]).0,
			Alice.into(),
		));
		assert_eq!(Balances::free_balance(&Alice.into()), 1000);
		assert_eq!(Vesting::vesting_balance(&Alice.into()), Some(800));

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

		// Old claim does not exist anymore
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Alice.into(),
				Alice.sign(&Airdrop::to_message(&Alice.into())[..]).0,
				Alice.into(),
			),
			Error::<Test>::HasNoClaim,
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
		assert_ok!(Airdrop::claim_raw(
			RuntimeOrigin::none(),
			Dave.into(),
			Dave.sign(&Airdrop::to_message(&Dave.into())[..]).0,
			Dave.into(),
		));
		assert_noop!(
			Airdrop::claim_raw(
				RuntimeOrigin::none(),
				Dave.into(),
				Dave.sign(&Airdrop::to_message(&Dave.into())[..]).0,
				Dave.into(),
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
