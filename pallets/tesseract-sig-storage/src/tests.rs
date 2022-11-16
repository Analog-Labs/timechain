use super::mock::*;
use crate::{types::{SignatureData, SignatureKey, TesseractRole, SignatureStorage}};
use frame_support::{assert_ok, traits::Randomness};
use frame_system::RawOrigin;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use pallet_randomness_collective_flip;


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
	let sig_key: Vec<u8> = "signature_key_1".as_bytes().to_owned();
	let hash_key = calculate_hash(&sig_key);
	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector));

		// Call the store signature extrinsic
		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(1).into(),
			hash_key.clone(),
			sig_data.clone()
		));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::signature_store(hash_key), Some(sig_data));
	});
}
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
#[test]
fn test_signature_storage() {
	let sig_key: Vec<u8> = "signature_key_1".as_bytes().to_owned();
	let hash_key = calculate_hash(&sig_key);
	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();
	let network_id = "12345".as_bytes();
	let block_height = 1234;
	let time_stamp = 1234;
	let storage_data = SignatureStorage::new(hash_key.clone(), sig_data.clone(), network_id.to_vec(), block_height, time_stamp);
	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector));

		// Call the store signature extrinsic
		assert_ok!(TesseractSigStorage::store_signature_data(
			RawOrigin::Signed(1).into(),
			storage_data.clone()
		));

		println!("key->{:?} - value->{:?}",hash_key, storage_data);
		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::signature_storage(hash_key), Some(storage_data));
	});
}
