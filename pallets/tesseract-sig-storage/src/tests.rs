use super::mock::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::ForeignEventId;

#[test]
fn test_signature_storage() {
	let r: u8 = rand::random();
	let sig_data: [u8; 64] = [r; 64];
	new_test_ext().execute_with(|| {
		let event_id: ForeignEventId = rand::random::<u128>().into();

		// TODO: update with proper tesseract and task creation after task management is in place
		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(1).into(),
			sig_data,
			event_id
		));

		assert_eq!(TesseractSigStorage::signature_storage(event_id), Some(sig_data));
	});
}
