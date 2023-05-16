use super::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use time_primitives::{ForeignEventId, TimeId};

// generate keypair
//  sp_core::sr25519::Pair
pub const ALICE: TimeId = TimeId::new([1u8; 32]);
pub const BOB: TimeId = TimeId::new([2u8; 32]);
pub const CHARLIE: TimeId = TimeId::new([3u8; 32]);
pub const DJANGO: TimeId = TimeId::new([4u8; 32]);

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

#[test]
fn test_register_shard_fails_if_member_len_not_supported() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				0, // setId is 0
				vec![ALICE, BOB, CHARLIE, DJANGO],
				Some(ALICE),
			),
			Error::<Test>::UnsupportedMembershipSize
		);
	});
}

#[test]
/// Currently supported sizes are 3, 5, 10
fn test_register_shard_works_for_supported_member_lengths() {
	new_test_ext().execute_with(|| {
		let mut members = vec![ALICE, BOB, CHARLIE];
		// supports 3
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			members.clone(),
			Some(ALICE),
		));

		// supports 5
		members.push(DJANGO);
		members.push(TimeId::new([5u8; 32]));
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			1, // setId is 0
			members.clone(),
			Some(ALICE),
		));

		// supports 10
		for i in 6..=10 {
			members.push(TimeId::new([i as u8; 32]));
		}
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			2, // setId is 0
			members,
			Some(ALICE),
		));
	});
}

#[test]
fn test_register_shard_fails_if_collector_not_in_membership() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				0, // setId is 0
				vec![ALICE, BOB, CHARLIE],
				Some(DJANGO),
			),
			Error::<Test>::CollectorNotInMembers
		);
	});
}

#[test]
fn test_register_shard_fails_if_shard_id_taken() {
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![ALICE, BOB, CHARLIE],
			Some(ALICE),
		));

		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				0, // setId is 0
				vec![ALICE, BOB, CHARLIE],
				Some(ALICE),
			),
			Error::<Test>::ShardAlreadyRegistered
		);
	});
}

#[test]
fn test_api_report_misbehavior_increments_report_count() {
	use sp_keystore::Keystore;
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let alice = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let bob = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![alice.into(), bob.into(), CHARLIE],
			Some(alice.into()),
		));

		// report 1st offence

		let alice_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &alice, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			CHARLIE,
			alice.into(),
			alice_report.into(),
		));
		let bob_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &bob, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		// report 2nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			CHARLIE,
			bob.into(),
			bob_report.into(),
		));
		// 2 reported offences
		assert_eq!(2, TesseractSigStorage::reported_offences(CHARLIE).unwrap().0);
	});
}

#[test]
fn test_api_report_misbehavior_moves_offences_to_committed() {
	use sp_keystore::Keystore;
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let alice = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let bob = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![alice.into(), bob.into(), CHARLIE],
			Some(alice.into()),
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let alice_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &alice, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			CHARLIE,
			alice.into(),
			alice_report.into(),
		));
		let bob_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &bob, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		// report 2nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			CHARLIE,
			bob.into(),
			bob_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(CHARLIE).is_some());
		// check commitment
		// report nth offence to move to commitment (see if count is tracked)
		//api_report_misbehavior
	});
}

#[test]
fn test_api_report_misbehavior_moves_offences_to_committed_without_purging_reported() {
	use sp_keystore::Keystore;
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let alice = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let bob = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![alice.into(), bob.into(), CHARLIE],
			Some(alice.into()),
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let alice_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &alice, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			CHARLIE,
			alice.into(),
			alice_report.into(),
		));
		let bob_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &bob, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		// report 2nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			CHARLIE,
			bob.into(),
			bob_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(CHARLIE).is_some());
		// is keeping reported offences in storage intentional or a bug
		assert!(TesseractSigStorage::reported_offences(CHARLIE).is_some());
	});
}
