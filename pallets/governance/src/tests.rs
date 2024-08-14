use crate::mock::*;

use polkadot_sdk::*;

use frame_support::{assert_noop, assert_ok};
use pallet_staking::ConfigOp;
use sp_runtime::traits::BadOrigin;
use sp_runtime::{Perbill, Percent};

use time_primitives::H256;

#[test]
fn success_system_admin() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);

		// Authorize upgrade and check result
		let hash = H256::random();
		assert_ok!(Governance::authorize_upgrade(RuntimeOrigin::signed(SystemAdmin::get()), hash));
		System::assert_last_event(
			frame_system::Event::UpgradeAuthorized {
				code_hash: hash,
				check_version: true,
			}
			.into(),
		);
	});
}

#[test]
fn success_staking_admin() {
	new_test_ext().execute_with(|| {
		// Set and check validator count
		assert_ok!(Governance::set_validator_count(RuntimeOrigin::signed(StakingAdmin::get()), 42));
		assert_eq!(Staking::validator_count(), 42);
	});
	new_test_ext().execute_with(|| {
		assert_ok!(Governance::set_staking_configs(
			RuntimeOrigin::signed(StakingAdmin::get()),
			ConfigOp::Set(1),
			ConfigOp::Set(1),
			ConfigOp::Set(1000),
			ConfigOp::Set(1000),
			ConfigOp::Set(Percent::one()),
			ConfigOp::Set(Perbill::one()),
			ConfigOp::Set(Percent::one()),
		));
		// FIXME: Check state change
	});
}

#[test]
fn fail_other() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Governance::authorize_upgrade(RuntimeOrigin::signed(Other::get()), H256::random()),
			BadOrigin
		);
		assert_noop!(
			Governance::set_validator_count(RuntimeOrigin::signed(Other::get()), 42),
			BadOrigin
		);
		assert_noop!(
			Governance::set_staking_configs(
				RuntimeOrigin::signed(Other::get()),
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
				ConfigOp::Noop,
			),
			BadOrigin
		);
	});
}
