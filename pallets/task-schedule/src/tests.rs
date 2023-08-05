use super::mock::*;
use frame_support::assert_ok;
use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;
use time_primitives::abstraction::{
	Function, ScheduleInput, ScheduleStatus, TaskSchedule as abs_TaskSchedule,
};
use time_primitives::Network;

#[test]
fn test_reward() {
	new_test_ext().execute_with(|| {
		let account_1: AccountId = acc_pub(1).into();
		let account_2: AccountId = acc_pub(2).into();

		let reward_1 = IndexerReward::get();
		let reward_2 = IndexerReward::get() * 2;

		let balance_1 = Balances::free_balance(&account_1);
		let balance_2 = Balances::free_balance(&account_2);

		TaskSchedule::on_initialize(1);

		assert_eq!(Balances::free_balance(&account_1), balance_1 + reward_1 - 1);
		assert_eq!(Balances::free_balance(&account_2), balance_2 + reward_2 - 2);

		// ensure score purged and no redundant reward
		TaskSchedule::on_initialize(2);

		assert_eq!(Balances::free_balance(&account_1), balance_1 + reward_1 - 1);
		assert_eq!(Balances::free_balance(&account_2), balance_2 + reward_2 - 2);
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
			hash: String::from("address"),
			start: 1,
			period: 1,
		};
		assert_ok!(TaskSchedule::create_task(RawOrigin::Signed(account.clone()).into(), input));

		let output = abs_TaskSchedule {
			owner: account.clone(),
			function: function.clone(),
			network: Network::Ethereum,
			cycle: 12,
			hash: String::from("address"),
			period: 1,
			start: 1,
		};
		let a = TaskSchedule::get_task(1_u64);
		let b = Some(output);
		assert_eq!(a, b);
		// update schedule
		// assert_ok!(TaskSchedule::submit_result(
		// 	RawOrigin::Signed(account.clone()).into(),
		// 	1,
		// 	1,
		// 	ScheduleStatus::Ok(1, taskE),
		// ));

		let output_update = abs_TaskSchedule {
			owner: account,
			function,
			network: Network::Ethereum,
			cycle: 12,
			hash: String::from("address"),
			start: 1,
			period: 1,
		};
		let a = TaskSchedule::get_task(1_u64);
		let b = Some(output_update);
		assert_eq!(a, b);
	});
}
