use super::mock::*;
use crate::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn test_schedule() {
	new_test_ext().execute_with(|| {
		let input = Schedule {
			task_id: ObjectId(1),
			shard_id: 1,
			cycle: 12,
			frequency: 1,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Initiated,
		};
		let account: AccountId = acc_pub(1).into();
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(account.clone()).into(),
			Some(1000),
			1,
			Some(1),
			1,
			acc_pub(1).into()
		));
		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(account.clone()).into(), input));

		let output = ScheduleOut {
			task_id: ObjectId(1),
			owner: account.clone(),
			shard_id: 1,
			start_execution_block: 0,
			executable_since: 1,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Initiated,
			frequency: 1,
		};
		let a = TaskSchedule::get_task_schedule(1_u64);
		let b = Some(output);
		assert_eq!(a, b);
		// update schedule
		assert_ok!(TaskSchedule::update_schedule(
			RawOrigin::Signed(account.clone()).into(),
			ScheduleStatus::Completed,
			1
		));

		let output_update = ScheduleOut {
			task_id: ObjectId(1),
			owner: account.clone(),
			shard_id: 1,
			start_execution_block: 0,
			executable_since: 1,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Completed,
			frequency: 1,
		};
		let a = TaskSchedule::get_task_schedule(1_u64);
		let b = Some(output_update);
		assert_eq!(a, b);
		// check update token usage
		let proxy_acc = PalletProxy::get_proxy_acc(&account).unwrap();
		match proxy_acc {
			Some(acc) => {
				let token_usage = 2;
				assert_eq!(acc.token_usage, token_usage);
			},
			None => print!("proxy account not exist"),
		}
	});
}

#[test]
fn test_payable_schedule() {
	new_test_ext().execute_with(|| {
		//Insert payable task schedule
		let input: PayableScheduleInput = PayableScheduleInput {
			task_id: ObjectId(1),
			shard_id: 1,
		};
		let account: AccountId = acc_pub(1).into();
		assert_ok!(PalletProxy::set_proxy_account(
			RawOrigin::Signed(account.clone()).into(),
			Some(1000),
			1,
			Some(1),
			1,
			account.clone()
		));
		assert_ok!(TaskSchedule::insert_payable_task_schedule(
			RawOrigin::Signed(account.clone()).into(),
			input
		));

		//get payable task schedule
		let output = PayableTaskSchedule {
			task_id: ObjectId(1),
			owner: account,
			shard_id: 1,
			executable_since: 1,
			status: ScheduleStatus::Initiated,
		};

		let a = TaskSchedule::get_payable_task_schedule(1_u64);
		let b = Some(output);
		assert_eq!(a, b);
	});
}
