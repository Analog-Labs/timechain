use crate::{mock::*, BalanceOf, Config, Error};

use polkadot_sdk::{
	frame_support::{self, traits::ExistenceRequirement},
	pallet_balances, sp_runtime,
};

use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::traits::{Get, Zero};

use time_primitives::{NetworkId, Task, TasksInterface};

const ETHEREUM: NetworkId = 0;
const TELEPORT_FEE: u128 = 12_500;
const TELEPORT_AMOUNT: u128 = 123_000;

fn register_gateway(network: NetworkId, block: u64) {
	Tasks::gateway_registered(network, block);
}

#[test]
fn first_teleport_below_ed() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bridge::do_register_network(ETHEREUM, TELEPORT_FEE, Default::default(),));

		let account = crate::Pallet::<Test>::account_id();
		let bridge_bal_before = <Test as Config>::Currency::free_balance(account);
		assert!(bridge_bal_before.is_zero());

		let ed: BalanceOf<Test> = <Test as pallet_balances::Config>::ExistentialDeposit::get();
		let below_ed = ed.saturating_sub(1);

		assert!(!below_ed.is_zero());

		assert_err!(
			Bridge::do_teleport(
				acc(1).into(),
				ETHEREUM,
				acc(2).into(),
				below_ed,
				ExistenceRequirement::KeepAlive
			),
			Error::<Test>::CannotReserveFunds,
		);
	})
}

#[test]
fn teleport_reserve_and_fee() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bridge::do_register_network(ETHEREUM, TELEPORT_FEE, Default::default(),));

		let account = crate::Pallet::<Test>::account_id();
		let user_bal_before = <Test as Config>::Currency::free_balance(acc(1));
		let bridge_bal_before = <Test as Config>::Currency::free_balance(&account);

		assert_ok!(Bridge::do_teleport(
			acc(1).into(),
			ETHEREUM,
			acc(2).into(),
			TELEPORT_AMOUNT,
			ExistenceRequirement::KeepAlive
		));

		let user_bal_after = <Test as Config>::Currency::free_balance(acc(1));
		let bridge_bal_after = <Test as Config>::Currency::free_balance(account);

		assert_eq!(user_bal_after, user_bal_before.saturating_sub(TELEPORT_AMOUNT + TELEPORT_FEE));
		assert_eq!(bridge_bal_after, bridge_bal_before.saturating_add(TELEPORT_AMOUNT));
	})
}

#[test]
fn teleport_creates_gmp_message() {
	new_test_ext().execute_with(|| {
		assert!(Tasks::get_task(0).is_none());
		register_gateway(ETHEREUM, 42);
		assert_eq!(Tasks::get_task(1), Some(Task::ReadGatewayEvents { blocks: 42..47 }));
		assert!(Tasks::get_task(2).is_none());

		assert_ok!(Bridge::do_register_network(ETHEREUM, Default::default(), Default::default(),));

		assert_ok!(Bridge::do_teleport(
			acc(1).into(),
			ETHEREUM,
			acc(2).clone().into(),
			123_000,
			ExistenceRequirement::KeepAlive
		));

		roll(1);

		assert_eq!(Tasks::get_task(2), Some(Task::SubmitGatewayMessage { batch_id: 0 }));
	})
}

#[test]
fn cannot_teleport_to_inactive_network() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bridge::do_teleport(
				acc(1).into(),
				ETHEREUM,
				acc(2).clone().into(),
				123_000,
				ExistenceRequirement::KeepAlive
			),
			Error::<Test>::NetworkDisabled,
		);
	})
}

#[test]
fn cannot_register_timechain_network() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Bridge::do_register_network(
				<Test as pallet_networks::Config>::TimechainNetworkId::get(),
				Default::default(),
				Default::default(),
			),
			Error::<Test>::NetworkAlreadyExists,
		);
	})
}
