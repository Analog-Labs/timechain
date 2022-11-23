use super::mock::*;
use crate::{types::{SignatureData, TesseractRole, SignatureStorage}};
use frame_support::{assert_ok};
use frame_system::RawOrigin;
use sp_core::hash::{H256};


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
	// let hash_key = TesseractSigStorage::gen_dna();
	
	// let hash_key = 0x0110111111111111111 as u8;
	// let hash_key = "signature_key_1".to_string();// calculate_hash(&sig_key);
	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector));
		// let hash_key = TesseractSigStorage::random_hash1();
		// Call the store signature extrinsic
		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(1).into(),
			sig_data.clone()
		));

		// Retreiving the signature stored via it's key and assert the result.
		// assert_eq!(TesseractSigStorage::signature_store(hash_key), Some(sig_data));
	});
}

#[test]
fn test_signature_storage() {
	// let sig_key: Vec<u8> = "signature_key_1".as_bytes().to_owned();
	// let hash_key = "signature_key_1".to_string();// calculate_hash(&sig_key);
	// let hash_key = H256::random();

	println!("storage_data -> ");
	// let hash_key2 = TesseractSigStorage::random_hash();
	// println!("storage_data -> ");
	// let hash_key = TesseractSigStorage::random_hash();
	// println!("storage_data -> {}",hash_key);
	// // let hash_key = TesseractSigStorage::gen_dna();
	// println!("storage_data -> {:?} ", hash_key);
	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();
	let network_id = "12345".as_bytes();
	let block_height = 1234;
	// let time_stamp =TesseractSigStorage::random_hash1();// TesseractSigStorage::gen_dna();//H256::random();
	// let storage_data = SignatureStorage::new(hash_key.clone(), sig_data.clone(), network_id.clone().to_vec(), block_height.clone(), time_stamp);
	new_test_ext().execute_with(|| {
		// We first add the Tesseract as a member with root privilege
		assert_ok!(TesseractSigStorage::add_member(RawOrigin::Root.into(), 1, TesseractRole::Collector));
		// println!("storage_data -> {:?} ", storage_data);
		// Call the store signature extrinsic
		assert_ok!(TesseractSigStorage::store_signature_data(
			RawOrigin::Signed(1).into(),
			sig_data.clone(),
			network_id.clone().to_vec(),
			block_height.clone()
		));
		// println!("storage_data -> {:?} ", storage_data);
		// println!("key->{:?} - value->{:?}",hash_key, storage_data);
		// Retreiving the signature stored via it's key and assert the result.
		// assert_eq!(TesseractSigStorage::signature_storage(hash_key), Some(storage_data));
	});
}
