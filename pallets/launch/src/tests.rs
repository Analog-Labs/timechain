//use crate::{mock::*, Error, Event, Something};
//use frame_support::{assert_noop, assert_ok};
#![allow(dead_code)]

use crate::mock::*;

use crate::{AirdropMigration, DepositMigration};

use time_primitives::{Balance, ANLOG};

// Operation allocation (launch budget) to pay for fees and test early integrations
const GENESIS: Balance = 3100 * ANLOG;

// First exchange allocations in preparation for TGE (v1 + v5)
const STAGE_1: Balance = (53_030_500 + 39_328_063) * ANLOG;

// First airdrop snapshot (v2 + v3 + v4)
const STAGE_2: Balance = 410_168_624 * ANLOG;
const STAGE_2_DIFF: Balance = STAGE_2 - 410_168_623_085_944_989_935;

// FIXME: Make this a check before deployment
const TOTAL_DEPLOYED: Balance = GENESIS + STAGE_1 + STAGE_2;
const TOTAL_TREASURY: Balance = STAGE_2_DIFF;

#[test]
fn data_v7_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v6 = DepositMigration::<Test>::new(crate::data::v7::DEPOSITS_PRELAUNCH_2);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum of endowment
		assert_eq!(v6.sum(), 226_449_338 * ANLOG);

		// Ensure non of the endowments causes an error event
		let _w = v6.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}

#[test]
fn data_v8_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v6 = AirdropMigration::<Test>::new(crate::data::v8::AIRDROP_VALIDATORS);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum of endowment
		assert_eq!(v6.sum(), 0);

		// Ensure non of the endowments causes an error event
		let _w = v6.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}

#[test]
fn data_v9_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v7 = DepositMigration::<Test>::new(crate::data::v9::DEPOSITS_TOKEN_GENESIS_EVENT);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum of endowment
		assert_eq!(v7.sum(), 0);

		// Ensure non of the endowments causes an error event
		let _w = v7.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}
