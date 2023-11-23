
use crate::mock::*;
use crate::{
	Error, Event};
    
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;


#[test]
fn test_create_task() {
	new_test_ext().execute_with(|| {
		// assert_ok!(Tasks::create_task(
		
		assert_eq!(true, false);
		// System::assert_last_event(Event::<Test>::TaskResult(0, 0, task_result).into());
	});
}