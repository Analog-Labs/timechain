use crate::mock::*;
use crate::Event;

use frame_support::assert_ok;
use frame_system::RawOrigin;
use polkadot_sdk::sp_runtime::BoundedVec;
use polkadot_sdk::{frame_support, frame_system};
use scale_codec::Encode;
use time_primitives::DmailMessage;

#[test]
fn test_dmail_event() {
	let to = BoundedVec::truncate_from("Self".encode());
	let path = BoundedVec::truncate_from("//self".encode());
	let sender: AccountId = [1; 32].into();
	let event = DmailMessage {
		owner: sender.clone(),
		to: to.clone(),
		path: path.clone(),
	};
	new_test_ext().execute_with(|| {
		assert_ok!(Dmail::send_email(
			RawOrigin::Signed(sender.clone()).into(),
			to.clone(),
			path.clone(),
		));
		System::assert_last_event(Event::<Test>::Message(event).into());
	});
}
