//use crate::{mock::*, Error, Event, Something};
//use frame_support::{assert_noop, assert_ok};
#![allow(dead_code)]

use crate::mock::*;

use crate::{AirdropMigration, DepositMigration};

use time_primitives::{Balance, ANLOG};

// Operation allocation (launch budget) to pay for fees and test early integrations
const GENESIS: Balance = 3100 * ANLOG;

// First exchange allocations in preparation for TGE
const STAGE_1: Balance = (53_030_500 + 39_328_063) * ANLOG;

// First airdrop snapshot
const STAGE_2: Balance = 410_168_623_085_944_989_935;
const STAGE_2_ROUNDED: Balance = 410_168_625 * ANLOG;
const STAGE_2_DIFF: Balance = STAGE_2_ROUNDED - STAGE_2;

// FIXME: Make this a check before deployment
const TOTAL_DEPLOYED: Balance = GENESIS + STAGE_1 + STAGE_2_ROUNDED;

#[test]
fn data_v5_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v5 = DepositMigration::<Test>::new(crate::data::v5::DEPOSITS_PRELAUNCH_1);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum of endowment
		assert_eq!(v5.sum(), 39_328_063 * ANLOG);

		// Ensure non of the endowments causes an error event
		let _w = v5.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}

#[test]
fn data_v6_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v6 = AirdropMigration::<Test>::new(crate::data::v6::AIRDROP_VALIDATORS);
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
fn data_v7_validation() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// Parse raw data of migration and check for any error event
		let v7 = DepositMigration::<Test>::new(crate::data::v7::DEPOSITS_TOKEN_GENESIS_EVENT);
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// Ensure correct sum of endowment
		assert_eq!(v7.sum(), 0);

		// Ensure non of the endowments causes an error event
		let _w = v7.execute();
		assert_eq!(System::read_events_for_pallet::<crate::Event::<Test>>().len(), 0);

		// TODO: Check weight
	});
}
