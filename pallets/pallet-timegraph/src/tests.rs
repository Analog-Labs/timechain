use super::mock::*;
use crate::{Error, Event};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;

#[test]
fn test_task() {
	new_test_ext().execute_with(|| {
		let account = 1;
		let query_id = 1;
		let balance: Balance = 2;
		let recipient = 3;
		let collection_id = 4;

		assert_ok!(Balances::force_set_balance(RawOrigin::Root.into(), account, 100));

		assert_ok!(PalletTimegraph::pay_querying(
			RawOrigin::Signed(account).into(),
			query_id,
			collection_id,
			balance,
			recipient
		));

		assert_eq!(PalletTimegraph::query_payment(query_id), Some((account, balance)));
		assert_eq!(PalletTimegraph::collection_earnings(collection_id), balance);

		assert!(System::events().iter().any(|event| {
			event.event
				== RuntimeEvent::PalletTimegraph(Event::QueryFeePaid(account, query_id, balance))
		}));

		assert_noop!(
			PalletTimegraph::pay_querying(
				RawOrigin::Signed(account).into(),
				query_id,
				collection_id,
				balance,
				recipient
			),
			Error::<Test>::QueryIdUsed
		);
	});
}
