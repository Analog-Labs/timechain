use crate as pallet_timegraph;
use crate::mock::*;
use crate::{Error, Event};

use polkadot_sdk::{frame_support, frame_system, pallet_balances};

use frame_support::{assert_noop, assert_ok, traits::Currency};
use frame_system::{Origin, RawOrigin};

#[test]
fn deposit_works() {
	new_test_ext().execute_with(|| {
		// Arrange
		let user = 1;
		let initial_balance = 1000;
		let deposit_amount = 500;

		// Ensure the user has enough balance
		Balances::make_free_balance_be(&user, initial_balance);

		// Act
		assert_ok!(Timegraph::deposit(RawOrigin::Signed(user).into(), deposit_amount));

		// Assert
		assert_eq!(Balances::reserved_balance(&user), deposit_amount);
		assert_eq!(Timegraph::next_deposit_sequence(&user), 1);
	});
}

#[test]
fn deposit_fails_with_zero_amount() {
	new_test_ext().execute_with(|| {
		// Arrange
		let user = 1;
		let deposit_amount = 0;

		// Act & Assert
		assert_noop!(
			Timegraph::deposit(RawOrigin::Signed(user).into(), deposit_amount),
			Error::<Test>::ZeroAmount
		);
	});
}

#[test]
fn deposit_fails_with_insufficient_balance() {
	new_test_ext().execute_with(|| {
		// Arrange
		let user = 1;
		let initial_balance = 100;
		let deposit_amount = 500;

		// Ensure the user has insufficient balance
		Balances::make_free_balance_be(&user, initial_balance);

		// Act & Assert
		assert_noop!(
			Timegraph::deposit(RawOrigin::Signed(user).into(), deposit_amount),
			pallet_balances::Error::<Test, _>::InsufficientBalance
		);
	});
}

#[test]
fn deposit_fails_with_sequence_overflow() {
	new_test_ext().execute_with(|| {
		let user = 1;

		let amount = 100;

		pallet_timegraph::NextDepositSequence::<Test>::insert(user, u64::MAX);
		assert_noop!(
			Timegraph::deposit(RawOrigin::Signed(user).into(), amount),
			Error::<Test>::SequenceNumberOverflow
		);
	});
}

// #[test]
// fn test_withdraw() {
// 	new_test_ext().execute_with(|| {x
// 		let sender = 1;
// 		let receiver = 2;
// 		let amount = 100;
// 		let init_sequence = 0;
// 		assert_eq!(Timegraph::next_withdrawal_sequence(sender), init_sequence);

// 		assert_noop!(
// 			Timegraph::withdraw(
// 				RawOrigin::Signed(sender).into(),
// 				sender,
// 				amount,
// 				init_sequence + 1
// 			),
// 			Error::<Test>::SenderSameWithReceiver
// 		);

// 		assert_noop!(
// 			Timegraph::withdraw(
// 				RawOrigin::Signed(sender).into(),
// 				receiver,
// 				0_u128,
// 				init_sequence + 1
// 			),
// 			Error::<Test>::ZeroAmount
// 		);

// 		assert_noop!(
// 			Timegraph::withdraw(RawOrigin::Signed(sender).into(), receiver, amount, init_sequence),
// 			Error::<Test>::WithDrawalSequenceMismatch
// 		);

// 		assert_noop!(
// 			Timegraph::withdraw(
// 				RawOrigin::Signed(sender).into(),
// 				receiver,
// 				amount,
// 				init_sequence + 2
// 			),
// 			Error::<Test>::WithDrawalSequenceMismatch
// 		);

// 		assert_ok!(Timegraph::withdraw(
// 			RuntimeOrigin::signed(sender),
// 			receiver,
// 			amount,
// 			init_sequence + 1
// 		));
// 		assert_eq!(Timegraph::next_withdrawal_sequence(sender), init_sequence + 1);

// 		System::assert_last_event(
// 			Event::<Test>::Withdrawal(sender, receiver, amount, init_sequence + 1).into(),
// 		);

// 		pallet_timegraph::NextWithdrawalSequence::<Test>::insert(sender, u64::MAX);
// 		assert_noop!(
// 			Timegraph::withdraw(RawOrigin::Signed(sender).into(), receiver, amount, init_sequence),
// 			Error::<Test>::SequenceNumberOverflow
// 		);
// 	});
// }
