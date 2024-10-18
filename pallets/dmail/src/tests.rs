use crate::mock::*;
use crate::Event;

use polkadot_sdk::{frame_support, frame_system, sp_runtime};

use frame_support::assert_ok;
use frame_system::RawOrigin;
use scale_codec::Encode;
use sp_runtime::BoundedVec;
use time_primitives::{DmailMessage, DmailPath, DmailTo};

#[test]
fn test_dmail_event() {
	let to: DmailTo = DmailTo(BoundedVec::truncate_from("Self".encode()));
	let path: DmailPath = DmailPath(BoundedVec::truncate_from("//self".encode()));
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
