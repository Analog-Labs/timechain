use crate::{mock::*, BridgedChain, Event};

use polkadot_sdk::*;

use frame_support::{
	assert_err, assert_ok,
	traits::{Currency, InspectLockableCurrency},
};
use sp_runtime::TokenError;

#[test]
fn bridge_works() {
	let a = make_account(1);
	let chain = BridgedChain::Ethereum;

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Check bridging request can be triggered
		assert_ok!(Bridge::bridge(
			RuntimeOrigin::signed(a.clone()),
			chain.clone(),
			[1u8; 20],
			BRIDGING_BALANCE
		));

		// Check that all funds were moved and locked
		assert_eq!(Balances::total_balance(&a), 0);
		assert_eq!(Balances::total_balance(&chain.account_id::<Test>()), BRIDGING_BALANCE);
		assert_eq!(
			Balances::balance_locked(chain.clone().lock_id(), &chain.account_id::<Test>()),
			BRIDGING_BALANCE
		);

		// Check that the correct event was triggered
		let events = System::read_events_for_pallet::<Event<Test>>();
		assert_eq!(events.len(), 1);
		assert_eq!(
			events[0],
			Event::BridgeRequest {
				chain,
				address: [1u8; 20],
				amount: BRIDGING_BALANCE,
			}
		);
	});
}

#[test]
fn bridge_checks_funds() {
	let b = make_account(2);
	let chain = BridgedChain::Base;

	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Check bridging request can not exceed balance
		assert_err!(
			Bridge::bridge(
				RuntimeOrigin::signed(b.clone()),
				chain.clone(),
				[2u8; 20],
				BRIDGING_BALANCE + 1
			),
			TokenError::FundsUnavailable
		);

		// Check that no funds were moved or locked
		assert_eq!(Balances::total_balance(&b), BRIDGING_BALANCE);
		assert_eq!(Balances::total_balance(&chain.account_id::<Test>()), 0,);
		assert_eq!(
			Balances::balance_locked(chain.clone().lock_id(), &chain.account_id::<Test>()),
			0
		);

		// Check that no event was triggered
		assert_eq!(System::read_events_for_pallet::<Event<Test>>().len(), 0);
	});
}
