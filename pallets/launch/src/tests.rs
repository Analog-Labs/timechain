//use crate::{mock::*, Error, Event, Something};
//use frame_support::{assert_noop, assert_ok};
use crate::mock::*;

use crate::{AirdropMigration, DepositMigration};

use time_primitives::ANLOG;

#[test]
fn data_v1_validation() {
	assert_eq!(
		DepositMigration::<Test>::new(crate::data::v1::DEPOSITS_PRELAUNCH).sum() as u128,
		92_292_563 * ANLOG,
	)
}

#[test]
fn data_v2_works() {
	AirdropMigration::<Test>::new(crate::data::v2::AIRDROP_SNAPSHOT_ONE);
}
