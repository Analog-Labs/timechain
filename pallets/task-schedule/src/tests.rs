use super::mock::*;
use crate::*;
use frame_support::assert_ok;
use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;
use time_primitives::abstraction::{
	Function, ScheduleInput, ScheduleStatus, TaskSchedule as abs_TaskSchedule,
};
use time_primitives::sharding::Network;

#[test]
fn test_reward() {
	new_test_ext().execute_with(|| {
		let account_1: AccountId = acc_pub(1).into();
		let account_2: AccountId = acc_pub(2).into();

		IndexerScore::<Test>::insert(&account_1, 1);
		IndexerScore::<Test>::insert(&account_2, 2);
		let reward_1 = IndexerReward::get();
		let reward_2 = IndexerReward::get() * 2;

		let balance_1 = Balances::free_balance(&account_1);
		let balance_2 = Balances::free_balance(&account_2);

		TaskSchedule::on_initialize(1);

		assert_eq!(Balances::free_balance(&account_1), balance_1 + reward_1);
		assert_eq!(Balances::free_balance(&account_2), balance_2 + reward_2);

		// ensure score purged and no redundant reward
		TaskSchedule::on_initialize(2);

		assert_eq!(Balances::free_balance(&account_1), balance_1 + reward_1);
		assert_eq!(Balances::free_balance(&account_2), balance_2 + reward_2);
	})
}

#[test]
fn test_schedule() {
	new_test_ext().execute_with(|| {
		let account: AccountId = acc_pub(1).into();
		let function = Function::EVMViewWithoutAbi {
			address: Default::default(),
			function_signature: Default::default(),
			input: Default::default(),
		};

		let input = ScheduleInput {
			network: Network::Ethereum,
			function: function.clone(),
			cycle: 12,
			frequency: 1,
			hash: String::from("address")
		};
		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(account.clone()).into(), input));

		let output = abs_TaskSchedule {
			owner: account.clone(),
			function: function.clone(),
			network: Network::Ethereum,
			cycle: 12,
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

		let output_update = abs_TaskSchedule {
			owner: account.clone(),
			function,
			network: Network::Ethereum,
			cycle: 12,
			hash: String::from("address"),
			status: ScheduleStatus::Completed,
			frequency: 1,
		};
		let a = TaskSchedule::get_task_schedule(1_u64);
		let b = Some(output_update);
		assert_eq!(a, b);
	});
}
