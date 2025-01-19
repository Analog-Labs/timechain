//use crate::{mock::*, Error, Event, Something};
//use frame_support::{assert_noop, assert_ok};
use crate::mock::*;

use crate::{AirdropMigration, DepositMigration};

use time_primitives::ANLOG;

#[test]
fn data_v1_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v1 = DepositMigration::<Test>::new(crate::data::v1::DEPOSITS_PRELAUNCH_0);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum of endowment
		assert_eq!(v1.sum(), 53_030_500 * ANLOG);

		// Ensure non of the endowments causes an error event
		let _w = v1.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}

#[test]
fn data_v2_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v2 = AirdropMigration::<Test>::new(crate::data::v2::AIRDROP_SNAPSHOT_ONE);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum
		assert_eq!(v2.sum(), 410_168_623_085_944_989_935);

		// Ensure non of the endowments causes an error event
		let _w = v2.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}
