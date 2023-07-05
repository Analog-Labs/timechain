use super::mock::*;
use crate::Error;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::{ProxyAccStatus, ProxyStatus};

#[test]
fn test_proxy_account() {
	new_test_ext().execute_with(|| {
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(10),
			10,
			Some(100),
			10,
			1
		));

		let output = ProxyAccStatus {
			owner: 1,
			max_token_usage: Some(10),
			token_usage: 10,
			max_task_execution: Some(100u32),
			task_executed: 10,
			status: ProxyStatus::Valid,
			proxy: 1,
		};
		assert_eq!(PalletProxy::get_proxy_status_store(1), Some(output));

		// update Proxy
		let expected_output = ProxyAccStatus {
			owner: 1,
			max_token_usage: Some(10),
			token_usage: 10,
			max_task_execution: Some(100u32),
			task_executed: 10,
			status: ProxyStatus::Suspended,
			proxy: 1,
		};
		let _ = PalletProxy::update_proxy_account(
			RawOrigin::Signed(1).into(),
			1,
			ProxyStatus::Suspended,
		);
		assert_eq!(PalletProxy::get_proxy_status_store(1), Some(expected_output));

		// remove Proxy
		let _ = PalletProxy::remove_proxy_account(RawOrigin::Signed(1).into(), 1);
		assert_eq!(PalletProxy::get_proxy_status_store(1), None);
	});
}

#[test]
fn test_set_proxy_account() {
	new_test_ext().execute_with(|| {
		// Set a proxy account with no maximum token usage and task execution limits
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			None,
			0,
			None,
			0,
			2
		));

		let output = ProxyAccStatus {
			owner: 1,
			max_token_usage: None,
			token_usage: 0,
			max_task_execution: None,
			task_executed: 0,
			status: ProxyStatus::Valid,
			proxy: 2,
		};
		assert_eq!(PalletProxy::get_proxy_status_store(2), Some(output));

		// Set a proxy account with specific maximum token usage and task execution limits
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(3).into(),
			Some(5),
			3,
			Some(50),
			30,
			4
		));

		let output = ProxyAccStatus {
			owner: 3,
			max_token_usage: Some(5),
			token_usage: 3,
			max_task_execution: Some(50u32),
			task_executed: 30,
			status: ProxyStatus::Valid,
			proxy: 4,
		};
		assert_eq!(PalletProxy::get_proxy_status_store(4), Some(output));
	});
}

#[test]
fn test_update_proxy_status() {
	new_test_ext().execute_with(|| {
		// Set a proxy account with an initial status to Suspended
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(10),
			10,
			Some(100),
			10,
			1
		));

		// Update the proxy account status to Suspended
		let _ = PalletProxy::update_proxy_account(
			RawOrigin::Signed(1).into(),
			1,
			ProxyStatus::Suspended,
		);
		let expected_output = ProxyAccStatus {
			owner: 1,
			max_token_usage: Some(10),
			token_usage: 10,
			max_task_execution: Some(100u32),
			task_executed: 10,
			status: ProxyStatus::Suspended,
			proxy: 1,
		};
		assert_eq!(PalletProxy::get_proxy_status_store(1), Some(expected_output));

		// Update the proxy account status to  Invalid
		let _ =
			PalletProxy::update_proxy_account(RawOrigin::Signed(1).into(), 1, ProxyStatus::Invalid);
		let expected_output = ProxyAccStatus {
			owner: 1,
			max_token_usage: Some(10),
			token_usage: 10,
			max_task_execution: Some(100u32),
			task_executed: 10,
			status: ProxyStatus::Invalid,
			proxy: 1,
		};
		assert_eq!(PalletProxy::get_proxy_status_store(1), Some(expected_output));
	});
}

#[test]
fn test_remove_proxy_account() {
	new_test_ext().execute_with(|| {
		// Set a proxy account
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(10),
			10,
			Some(100),
			10,
			1
		));

		// Remove the proxy account
		let _ = PalletProxy::remove_proxy_account(RawOrigin::Signed(1).into(), 1);
		assert_eq!(PalletProxy::get_proxy_status_store(1), None);
	});
}

#[test]
fn test_proxy_account_not_owner() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PalletProxy::set_proxy_account(
				RawOrigin::Signed(1).into(),
				Some(10),
				10,
				Some(100),
				10,
				2
			),
			Error::<Test>::NoPermission
		);
	});
}

#[test]
fn test_proxy_account_already_exists() {
	new_test_ext().execute_with(|| {
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(1).into(),
			Some(10),
			10,
			Some(100),
			10,
			2
		));

		assert_noop!(
			PalletProxy::set_proxy_account(RawOrigin::Signed(1).into(), Some(5), 5, Some(50), 5, 2),
			Error::<Test>::ProxyAlreadyExists
		);
	});
}

#[test]
fn test_proxy_account_not_exist() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			PalletProxy::update_proxy_account(
				RawOrigin::Signed(1).into(),
				2,
				ProxyStatus::Suspended,
			),
			Error::<Test>::ProxyNotExist
		);

		assert_noop!(
			PalletProxy::remove_proxy_account(RawOrigin::Signed(1).into(), 2),
			Error::<Test>::ProxyNotExist
		);
	});
}
