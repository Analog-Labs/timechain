use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{ProxyAccInput, ProxyAccStatus, ProxyStatus};

#[test]
fn test_schedule() {
	new_test_ext().execute_with(|| {
		let input = ProxyAccInput {
			max_token_usage: Some(10),
			token_usage: 10,
			max_task_execution: Some(10),
			task_executed: 10,
		};
		assert_ok!(PalletProxy::set_delegate_account(RawOrigin::Signed(1).into(), input));

		let output = ProxyAccStatus {
			owner: RawOrigin::Signed(1),
			max_token_usage: Some(10),
			token_usage: 10,
			max_task_execution: Some(100u32),
			task_executed: 10,
			status: ProxyStatus::Valid,
		};
		assert_eq!(PalletProxy::get_proxy_status_store(RawOrigin::Signed(1)), Some(output));
	});
}
