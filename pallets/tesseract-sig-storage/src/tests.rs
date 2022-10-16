use super::mock::*;
use crate::{types::{SignatureData, SignatureKey, TesseractRole}};
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[test]
fn it_works_adding_tesseract_member() {
	new_test_ext().execute_with(|| {
		// Call the tesseract member extrinsic
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::tesseract_members(1), Some(TesseractRole::Collector));
	});
}

#[test]
fn it_works_removing_tesseract_member() {
	new_test_ext().execute_with(|| {
		// Call the tesseract member extrinsic
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector,));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::tesseract_members(1), Some(TesseractRole::Collector));

		// Call the tesseract member extrinsic
		assert_ok!(TesseractSigStorage::remove_member(RawOrigin::Root.into(), 1,));
		// Checking that the member has been rmoved
		assert_eq!(TesseractSigStorage::tesseract_members(1), None);
	});
}

#[test]
fn it_works_storing_and_get_signature_data() {
	let sig_key: SignatureKey = "signature_key_1".as_bytes().to_owned();

	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector));

		// Call the store signature extrinsic
		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(1).into(),
			sig_key.clone(),
			sig_data.clone()
		));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::signature_store(sig_key), Some(sig_data));
	});
}
