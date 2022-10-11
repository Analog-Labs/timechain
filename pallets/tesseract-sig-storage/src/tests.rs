use crate::{mock::*, SignatureData, SignatureKey};
use frame_support::assert_ok;

#[test]
fn it_works_storing_and_get_signature_data() {
	let sig_key: SignatureKey = "signature_key_1".as_bytes().to_owned();

	let sig_data: SignatureData = "this_is_the_signature_data_1".as_bytes().to_owned();

	new_test_ext().execute_with(|| {
		// Call the store signature extrinsic
		assert_ok!(TesseractSigStorage::store_signature(
			Origin::signed(1),
			sig_key.clone(),
			sig_data.clone()
		));

		// Retreiving the signature stored via it's key and assert the result.
		assert_eq!(TesseractSigStorage::signature_store(sig_key), Some(sig_data));
	});
}
