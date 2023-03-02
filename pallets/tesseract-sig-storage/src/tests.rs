use super::mock::*;
use crate::types::{SignatureData, SignatureStorage, TesseractRole};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use time_primitives::ForeignEventId;

#[test]
fn it_works_adding_tesseract_member() {
	new_test_ext().execute_with(|| {
		// Call the tesseract member extrinsic
		assert_ok!(TesseractSigStorage::add_member(
			RawOrigin::Root.into(),
			1,
			TesseractRole::Collector
		));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::tesseract_members(1), Some(TesseractRole::Collector));
	});
}

#[test]
fn it_works_removing_tesseract_member() {
	new_test_ext().execute_with(|| {
		// Call the tesseract member extrinsic
		assert_ok!(TesseractSigStorage::add_member(
			RawOrigin::Root.into(),
			1,
			TesseractRole::Collector,
		));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::tesseract_members(1), Some(TesseractRole::Collector));

		// Call the tesseract member extrinsic
		assert_ok!(TesseractSigStorage::remove_member(RawOrigin::Root.into(), 1,));
		// Checking that the member has been rmoved
		assert_eq!(TesseractSigStorage::tesseract_members(1), None);
	});
}

#[test]
fn test_signature_storage() {
	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();
	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(TesseractSigStorage::add_member(
			RawOrigin::Root.into(),
			1,
			TesseractRole::Collector
		));

		let event_id: ForeignEventId = rand::random::<u128>().into();

		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(1).into(),
			sig_data.clone(),
			event_id
		));

		let storage_data = SignatureStorage::new(sig_data.clone(), Timestamp::now());

		assert_eq!(TesseractSigStorage::signature_storage(event_id), Some(storage_data));
	});
}
