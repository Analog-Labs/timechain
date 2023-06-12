use super::mock::*;
use frame_support::assert_ok;
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
