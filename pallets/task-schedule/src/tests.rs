use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::{
	abstraction::{
		ObjectId, PayableScheduleInput, PayableTaskSchedule, ScheduleInput as Schedule,
		ScheduleStatus, TaskSchedule as ScheduleOut, Validity,
	},
	ProxyAccInput,
};

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
		};
		let proxy_data = ProxyAccInput {
			proxy: 1,
			max_token_usage: Some(1000),
			token_usage: 1,
			max_task_execution: Some(1),
			task_executed: 1,
		};
		let account = 1;
		let _ = PalletProxy::set_proxy_account(RawOrigin::Signed(account).into(), proxy_data);
		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(account).into(), input));

		let output = ScheduleOut {
			task_id: ObjectId(1),
			owner: 1,
			shard_id: 1,
			cycle: 12,
			frequency: 1,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			start_execution_block: 0,
			last_execution_block: 0,
			status: ScheduleStatus::Initiated,
		};
		assert_eq!(TaskSchedule::get_task_schedule(account as u64), Some(output));
		// update schedule
		assert_ok!(TaskSchedule::update_schedule(
			RawOrigin::Signed(account).into(),
			ScheduleStatus::Completed,
			1
		));

		let output_update = ScheduleOut {
			task_id: ObjectId(1),
			owner: account,
			shard_id: 1,
			cycle: 12,
			frequency: 1,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			start_execution_block: 0,
			last_execution_block: 0,
			status: ScheduleStatus::Completed,
		};
		assert_eq!(TaskSchedule::get_task_schedule(account as u64), Some(output_update));
		// check update token usage
		let proxy_acc = PalletProxy::get_proxy_acc(account).unwrap();
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
		let proxy_data = ProxyAccInput {
			proxy: 1,
			max_token_usage: Some(1000),
			token_usage: 1,
			max_task_execution: Some(1),
			task_executed: 1,
		};
		let account = 1;
		let _ = PalletProxy::set_proxy_account(RawOrigin::Signed(account).into(), proxy_data);
		assert_ok!(TaskSchedule::insert_payable_task_schedule(
			RawOrigin::Signed(account).into(),
			input
		));

		//get payable task schedule
		let output = PayableTaskSchedule {
			task_id: ObjectId(1),
			owner: 1,
			shard_id: 1,
			status: ScheduleStatus::Initiated,
		};
		assert_eq!(TaskSchedule::get_payable_task_schedule(account as u64), Some(output));
	});
}
