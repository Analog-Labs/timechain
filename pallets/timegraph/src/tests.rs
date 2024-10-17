use crate as pallet_timegraph;
use crate::mock::*;
use crate::Error;

use polkadot_sdk::{frame_support, frame_system, pallet_balances, sp_runtime};

use frame_support::{assert_noop, assert_ok, traits::Currency};
use frame_system::RawOrigin;
use sp_runtime::traits::BadOrigin;

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
		assert_eq!(Balances::reserved_balance(user), deposit_amount);
		assert_eq!(Timegraph::next_deposit_sequence(user), 1);
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

#[test]
fn test_withdraw_success() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let origin = RawOrigin::Signed(user);
		let amount = 1000;
		assert_ok!(Timegraph::deposit(
			RawOrigin::Signed(user).into(),
			amount + pallet_timegraph::Threshold::<Test>::get(),
		));
		let reserved = <Test as crate::pallet::Config>::Currency::reserved_balance(user);
		assert_eq!(reserved, amount + pallet_timegraph::Threshold::<Test>::get(),);
		assert_ok!(Timegraph::withdraw(origin.into(), amount));
		let reserved = <Test as crate::pallet::Config>::Currency::reserved_balance(user);
		assert_eq!(reserved, pallet_timegraph::Threshold::<Test>::get());
	});
}

#[test]
fn test_withdraw_zero_amount() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let origin = RawOrigin::Signed(user);
		// let amount: BalanceOf<Test> = 0;
		assert_noop!(Timegraph::withdraw(origin.into(), 0_u32.into()), Error::<Test>::ZeroAmount);
	});
}

#[test]
fn test_withdraw_over_reserve() {
	new_test_ext().execute_with(|| {
		let user = 1;
		let origin = RawOrigin::Signed(user);
		let amount = 1_000;
		assert_noop!(
			Timegraph::withdraw(origin.into(), amount),
			Error::<Test>::WithdrawalAmountOverReserve
		);
	});
}

#[test]
fn test_withdraw_bad_origin() {
	new_test_ext().execute_with(|| {
		let origin = RawOrigin::None;
		let amount = 1000;
		assert_noop!(Timegraph::withdraw(origin.into(), amount), BadOrigin);
	});
}

#[test]
fn transfer_to_pool_works() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		let user_account = 2;
		let transfer_amount = 500;

		// Set the timegraph account
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);

		assert_ok!(Timegraph::deposit(
			RawOrigin::Signed(user_account).into(),
			transfer_amount + pallet_timegraph::Threshold::<Test>::get(),
		));

		// Act
		assert_ok!(Timegraph::transfer_to_pool(
			RawOrigin::Signed(timegraph_account).into(),
			user_account,
			transfer_amount
		));

		// Assert
		assert_eq!(
			Balances::reserved_balance(user_account),
			pallet_timegraph::Threshold::<Test>::get()
		);
	});
}

#[test]
fn transfer_to_pool_fails_with_non_timegraph_origin() {
	new_test_ext().execute_with(|| {
		// Arrange
		let non_timegraph_account = 3;
		let user_account = 2;
		let transfer_amount = 500;

		// Act & Assert
		assert_noop!(
			Timegraph::transfer_to_pool(
				RawOrigin::Signed(non_timegraph_account).into(),
				user_account,
				transfer_amount
			),
			Error::<Test>::SenderIsNotTimegraph
		);
	});
}

#[test]
fn transfer_to_pool_fails_with_insufficient_reserved_balance() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		let user_account = 2;
		let initial_reserved_balance = 300;
		let transfer_amount = 500;

		// Set the timegraph account
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);

		assert_ok!(Timegraph::deposit(
			RawOrigin::Signed(user_account).into(),
			initial_reserved_balance + pallet_timegraph::Threshold::<Test>::get(),
		));

		// Act & Assert
		assert_noop!(
			Timegraph::transfer_to_pool(
				RawOrigin::Signed(timegraph_account).into(),
				user_account,
				transfer_amount + pallet_timegraph::Threshold::<Test>::get()
			),
			Error::<Test>::NotWithdrawalRequired
		);
	});
}

#[test]
fn transfer_award_to_user_works() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		let user_account = 2;
		let reward_pool_account = 3;
		let initial_pool_balance = 1000;
		let transfer_amount = 500;

		// Set the timegraph and reward pool accounts
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);
		pallet_timegraph::RewardPoolAccount::<Test>::set(reward_pool_account);

		// Ensure the reward pool has enough balance
		Balances::make_free_balance_be(&reward_pool_account, initial_pool_balance);

		// Act
		assert_ok!(Timegraph::transfer_award_to_user(
			RawOrigin::Signed(timegraph_account).into(),
			user_account,
			transfer_amount
		));

		// Assert
		assert_eq!(Balances::reserved_balance(user_account), transfer_amount);
		assert_eq!(
			Balances::free_balance(reward_pool_account),
			initial_pool_balance - transfer_amount
		);
	});
}

#[test]
fn transfer_award_to_user_fails_with_insufficient_pool_balance() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		let user_account = 2;
		let reward_pool_account = 3;
		let initial_pool_balance = 300;
		let transfer_amount = 500;

		// Set the timegraph and reward pool accounts
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);
		pallet_timegraph::RewardPoolAccount::<Test>::set(reward_pool_account);

		// Ensure the reward pool has insufficient balance
		Balances::make_free_balance_be(&reward_pool_account, initial_pool_balance);

		// Act & Assert
		assert_noop!(
			Timegraph::transfer_award_to_user(
				RawOrigin::Signed(timegraph_account).into(),
				user_account,
				transfer_amount
			),
			Error::<Test>::RewardPoolOutOfBalance
		);
	});
}

#[test]
fn transfer_award_to_user_fails_when_transferring_to_reward_pool_account() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		let reward_pool_account = 3;
		let transfer_amount = 500;

		// Set the timegraph and reward pool accounts
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);
		pallet_timegraph::RewardPoolAccount::<Test>::set(reward_pool_account);

		// Act & Assert
		assert_noop!(
			Timegraph::transfer_award_to_user(
				RawOrigin::Signed(timegraph_account).into(),
				reward_pool_account,
				transfer_amount
			),
			Error::<Test>::RewardToSameAccount
		);
	});
}

#[test]
fn set_timegraph_account_works() {
	new_test_ext().execute_with(|| {
		// Arrange
		let new_timegraph_account = 1;
		let root_origin = RawOrigin::Root;

		// Act
		assert_ok!(Timegraph::set_timegraph_account(root_origin.into(), new_timegraph_account));

		// Assert
		assert_eq!(pallet_timegraph::TimegraphAccount::<Test>::get(), new_timegraph_account);
	});
}

#[test]
fn set_timegraph_account_fails_with_same_account() {
	new_test_ext().execute_with(|| {
		// Arrange
		let existing_timegraph_account = 1;
		let root_origin = RawOrigin::Root;

		// Set the initial timegraph account
		pallet_timegraph::TimegraphAccount::<Test>::set(existing_timegraph_account);

		// Act & Assert
		assert_noop!(
			Timegraph::set_timegraph_account(root_origin.into(), existing_timegraph_account),
			Error::<Test>::SameTimegraphAccount
		);
	});
}

#[test]
fn set_timegraph_account_fails_with_non_root_origin() {
	new_test_ext().execute_with(|| {
		// Arrange
		let new_timegraph_account = 1;
		let non_root_origin = RawOrigin::Signed(2); // Non-root account

		// Act & Assert
		assert_noop!(
			Timegraph::set_timegraph_account(non_root_origin.into(), new_timegraph_account),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_reward_pool_account_works() {
	new_test_ext().execute_with(|| {
		// Arrange
		let new_reward_pool_account = 6;
		let root_origin = RawOrigin::Root;

		// Act
		assert_ok!(Timegraph::set_reward_pool_account(root_origin.into(), new_reward_pool_account));

		// Assert
		assert_eq!(pallet_timegraph::RewardPoolAccount::<Test>::get(), new_reward_pool_account);
	});
}

#[test]
fn set_reward_pool_account_fails_with_same_account() {
	new_test_ext().execute_with(|| {
		// Arrange
		let existing_reward_pool_account = 1;
		let root_origin = RawOrigin::Root;

		// Set the initial reward pool account
		pallet_timegraph::RewardPoolAccount::<Test>::set(existing_reward_pool_account);

		// Act & Assert
		assert_noop!(
			Timegraph::set_reward_pool_account(root_origin.into(), existing_reward_pool_account),
			Error::<Test>::SameRewardPoolAccount
		);
	});
}

#[test]
fn set_reward_pool_account_fails_with_non_root_origin() {
	new_test_ext().execute_with(|| {
		// Arrange
		let new_reward_pool_account = 1;
		let non_root_origin = RawOrigin::Signed(2); // Non-root account

		// Act & Assert
		assert_noop!(
			Timegraph::set_reward_pool_account(non_root_origin.into(), new_reward_pool_account),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn set_threshold_works() {
	new_test_ext().execute_with(|| {
		// Arrange
		let new_threshold = 2000;
		let root_origin = RawOrigin::Root;

		// Act
		assert_ok!(Timegraph::set_threshold(root_origin.into(), new_threshold));

		// Assert
		assert_eq!(pallet_timegraph::Threshold::<Test>::get(), new_threshold);
	});
}

#[test]
fn set_threshold_fails_with_same_value() {
	new_test_ext().execute_with(|| {
		// Arrange
		let initial_threshold = 1000;
		let root_origin = RawOrigin::Root;

		// Set the initial threshold
		pallet_timegraph::Threshold::<Test>::set(initial_threshold);

		// Act & Assert
		assert_noop!(
			Timegraph::set_threshold(root_origin.into(), initial_threshold),
			Error::<Test>::SameThreshold
		);
	});
}

#[test]
fn set_threshold_fails_with_non_root_origin() {
	new_test_ext().execute_with(|| {
		// Arrange
		let new_threshold = 2000;
		let non_root_origin = RawOrigin::Signed(1);

		// Act & Assert
		assert_noop!(
			Timegraph::set_threshold(non_root_origin.into(), new_threshold),
			sp_runtime::DispatchError::BadOrigin
		);
	});
}

#[test]
fn ensure_timegraph_works_with_correct_origin() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);

		// Act & Assert
		assert_ok!(Timegraph::ensure_timegraph(RawOrigin::Signed(timegraph_account).into()));
	});
}

#[test]
fn ensure_timegraph_fails_with_incorrect_origin() {
	new_test_ext().execute_with(|| {
		// Arrange
		let timegraph_account = 1;
		let non_timegraph_account = 2;
		pallet_timegraph::TimegraphAccount::<Test>::set(timegraph_account);

		// Act & Assert
		assert_noop!(
			Timegraph::ensure_timegraph(RawOrigin::Signed(non_timegraph_account).into()),
			Error::<Test>::SenderIsNotTimegraph
		);
	});
}
