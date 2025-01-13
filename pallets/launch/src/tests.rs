//use crate::{mock::*, Error, Event, Something};
//use frame_support::{assert_noop, assert_ok};
use crate::mock::*;

use crate::{AirdropMigration, DepositMigration};

#[test]
fn data_v1_works() {
	DepositMigration::<Test>::new(crate::data::v1::DEPOSITS_PRELAUNCH);
}

#[test]
fn data_v2_works() {
	AirdropMigration::<Test>::new(crate::data::v2::AIRDROP_SNAPSHOT_ONE);
}
