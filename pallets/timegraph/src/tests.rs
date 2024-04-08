use crate as pallet_timegraph;
use crate::mock::*;
use crate::{Error, Event};

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

#[test]
fn test_deposit() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let receiver = 2;
		let amount = 100;
		let init_sequence = 0;
		assert_eq!(Timegraph::next_deposit_sequence(sender), init_sequence);
		assert_noop!(
			Timegraph::deposit(RawOrigin::Signed(sender).into(), sender, amount),
			Error::<Test>::SenderSameWithReceiver
		);

		assert_noop!(
			Timegraph::deposit(RawOrigin::Signed(sender).into(), receiver, 0_u128),
			Error::<Test>::ZeroAmount
		);

		assert_ok!(Timegraph::deposit(RuntimeOrigin::signed(sender), receiver, amount));

		System::assert_last_event(
			Event::<Test>::Deposit(sender, receiver, amount, init_sequence + 1).into(),
		);

		pallet_timegraph::NextDepositSequence::<Test>::insert(receiver, u64::MAX);
		assert_noop!(
			Timegraph::deposit(RawOrigin::Signed(sender).into(), receiver, amount),
			Error::<Test>::SequenceNumberOverflow
		);
	});
}

#[test]
fn test_withdraw() {
	new_test_ext().execute_with(|| {
		let sender = 1;
		let receiver = 2;
		let amount = 100;
		let init_sequence = 0;
		assert_eq!(Timegraph::next_withdrawal_sequence(sender), init_sequence);

		assert_noop!(
			Timegraph::withdraw(
				RawOrigin::Signed(sender).into(),
				sender,
				amount,
				init_sequence + 1
			),
			Error::<Test>::SenderSameWithReceiver
		);

		assert_noop!(
			Timegraph::withdraw(
				RawOrigin::Signed(sender).into(),
				receiver,
				0_u128,
				init_sequence + 1
			),
			Error::<Test>::ZeroAmount
		);

		assert_noop!(
			Timegraph::withdraw(RawOrigin::Signed(sender).into(), receiver, amount, init_sequence),
			Error::<Test>::WithDrawalSequenceMismatch
		);

		assert_noop!(
			Timegraph::withdraw(
				RawOrigin::Signed(sender).into(),
				receiver,
				amount,
				init_sequence + 2
			),
			Error::<Test>::WithDrawalSequenceMismatch
		);

		assert_ok!(Timegraph::withdraw(
			RuntimeOrigin::signed(sender),
			receiver,
			amount,
			init_sequence + 1
		));
		assert_eq!(Timegraph::next_withdrawal_sequence(sender), init_sequence + 1);

		System::assert_last_event(
			Event::<Test>::Withdrawal(sender, receiver, amount, init_sequence + 1).into(),
		);

		pallet_timegraph::NextWithdrawalSequence::<Test>::insert(sender, u64::MAX);
		assert_noop!(
			Timegraph::withdraw(RawOrigin::Signed(sender).into(), receiver, amount, init_sequence),
			Error::<Test>::SequenceNumberOverflow
		);
	});
}
