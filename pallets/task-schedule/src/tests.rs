use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::abstraction::{
	ObjectId, ScheduleInput as Schedule, ScheduleStatus, TaskSchedule as ScheduleOut, Validity,
};

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
		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(1).into(), input));

		let output = ScheduleOut {
			task_id: ObjectId(1),
			owner: 1,
			shard_id: 1,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Initiated,
		};
		assert_eq!(TaskSchedule::get_task_schedule(1), Some(output));
		// update schedule
		assert_ok!(TaskSchedule::update_schedule(
			RawOrigin::Signed(1).into(),
			ScheduleStatus::Completed,
			1
		));

		let output_update = ScheduleOut {
			task_id: ObjectId(1),
			owner: 1,
			shard_id: 1,
			cycle: 12,
			validity: Validity::Seconds(10),
			hash: String::from("address"),
			status: ScheduleStatus::Completed,
		};
		assert_eq!(TaskSchedule::get_task_schedule(1), Some(output_update));
	});
}
