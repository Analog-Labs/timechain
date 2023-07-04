use super::mock::*;
use crate::{Error, Event};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

#[test]
fn test_task() {
	new_test_ext().execute_with(|| {
		let query_id = 1;
		let balance: Balance = 2;
		let recipient = 3;

		assert_ok!(Balances::force_set_balance(RawOrigin::Root.into(), 1, 100));

		assert_ok!(PalletTimegraph::pay_querying(
			RawOrigin::Signed(1).into(),
			query_id,
			balance,
			recipient
		));

		assert!(System::events().iter().any(|event| {
			event.event == RuntimeEvent::PalletTimegraph(Event::QueryFeePaid(1, query_id, balance))
		}));

		assert_noop!(
			PalletTimegraph::pay_querying(
				RawOrigin::Signed(1).into(),
				query_id,
				balance,
				recipient
			),
			Error::<Test>::QueryIdUsed
		);
	});
}
