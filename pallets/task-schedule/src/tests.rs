use super::mock::*;
use crate::*;
use frame_support::assert_ok;
use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;
use time_primitives::abstraction::{
	ObjectId, PayableScheduleInput, PayableTaskSchedule, ScheduleInput as Schedule, ScheduleStatus,
	TaskSchedule as ScheduleOut, Validity,
};

#[test]
fn test_reward() {
	new_test_ext().execute_with(|| {
		let account_1: AccountId = acc_pub(1).into();
		let account_2: AccountId = acc_pub(2).into();

		AllIndexer::<Test>::insert(&account_1, 1);
		AllIndexer::<Test>::insert(&account_2, 2);
		let reward_1 = IndexerReward::get();
		let reward_2 = IndexerReward::get() * 2;

		let balance_1 = Balances::free_balance(&account_1);
		let balance_2 = Balances::free_balance(&account_2);

		TaskSchedule::on_initialize(1);

		assert_eq!(Balances::free_balance(&account_1), balance_1 + reward_1);
		assert_eq!(Balances::free_balance(&account_2), balance_2 + reward_2);
	})
}

#[test]
fn test_schedule() {
	new_test_ext().execute_with(|| {
		let input = Schedule {
			task_id: ObjectId(1),
			shard_id: 1,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
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
			start_block: 0,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Initiated,
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
			start_block: 0,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Completed,
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
			status: ScheduleStatus::Initiated,
		};

		let a = TaskSchedule::get_payable_task_schedule(1_u64);
		let b = Some(output);
		assert_eq!(a, b);
	});
}
