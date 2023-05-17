use super::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_keystore::Keystore;
use time_primitives::{ForeignEventId, TimeId};

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
		assert_eq!(2, TesseractSigStorage::commited_offences(CHARLIE).unwrap().0);
	});
}

#[test]
fn test_api_report_misbehavior_updates_reporters() {
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
		// alice and bob are reporters
		assert!(TesseractSigStorage::commited_offences(CHARLIE)
			.unwrap()
			.1
			.contains(&alice.into()));
		assert!(TesseractSigStorage::commited_offences(CHARLIE).unwrap().1.contains(&bob.into()));
	});
}

#[test]
/// Moves offences to committed such that reported offences is emptied
fn test_api_report_misbehavior_moves_offences_to_committed() {
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
		let mut expected_committed_offences =
			TesseractSigStorage::reported_offences(CHARLIE).unwrap();
		expected_committed_offences.0 += 1;
		expected_committed_offences.1.insert(bob.into());
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
		assert_eq!(
			expected_committed_offences,
			TesseractSigStorage::commited_offences(CHARLIE).unwrap()
		);
		// remove reported_offences from storage once moved to commited_offences
		assert!(TesseractSigStorage::reported_offences(CHARLIE).is_none());
	});
}

#[test]
fn test_api_report_misbehavior_for_group_len_5() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let charlie = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let david = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let edward = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![ALICE, BOB, charlie.into(), david.into(), edward.into()],
			Some(charlie.into()),
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let charlie_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &charlie, ALICE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			charlie.into(),
			charlie_report.into(),
		));
		let david_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &david, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 2nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			david.into(),
			david_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(ALICE).is_none());
		let mut expected_committed_offences =
			TesseractSigStorage::reported_offences(ALICE).unwrap();
		expected_committed_offences.0 += 1;
		expected_committed_offences.1.insert(edward.into());
		let edward_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &edward, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 3nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			edward.into(),
			edward_report.into(),
		));
		assert_eq!(
			expected_committed_offences,
			TesseractSigStorage::commited_offences(ALICE).unwrap()
		);
	});
}

#[test]
fn test_api_report_misbehavior_for_group_len_10() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let edward = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let frank = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let greg = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let hank = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let indigo = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let jared = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![
				ALICE,
				BOB,
				CHARLIE,
				DJANGO,
				edward.into(),
				frank.into(),
				greg.into(),
				hank.into(),
				indigo.into(),
				jared.into()
			],
			Some(edward.into()),
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let edward_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &edward, ALICE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			edward.into(),
			edward_report.into(),
		));
		let frank_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &frank, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 2nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			frank.into(),
			frank_report.into(),
		));
		let greg_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &greg, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 3nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			greg.into(),
			greg_report.into(),
		));
		let hank_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &hank, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 4th offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			hank.into(),
			hank_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(ALICE).is_none());
		let mut expected_committed_offences =
			TesseractSigStorage::reported_offences(ALICE).unwrap();
		expected_committed_offences.0 += 1;
		expected_committed_offences.1.insert(indigo.into());
		let indigo_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &indigo, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 5th offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			indigo.into(),
			indigo_report.into(),
		));
		assert!(TesseractSigStorage::reported_offences(ALICE).is_none());
		assert_eq!(
			expected_committed_offences,
			TesseractSigStorage::commited_offences(ALICE).unwrap()
		);
	});
}

#[test]
/// Once moved to committed, cannot be reported again until this is handled
/// i.e. until slashing punishment is executed
fn cannot_report_offence_if_already_committed_offender() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let bob = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let charlie = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let david = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	let edward = keystore
		.sr25519_generate_new(time_primitives::KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			0, // setId is 0
			vec![ALICE, bob.into(), charlie.into(), david.into(), edward.into()],
			Some(charlie.into()),
		));

		// report 1st offence
		let bob_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &bob, ALICE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			bob.into(),
			bob_report.into(),
		));
		let charlie_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &charlie, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 2nd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			charlie.into(),
			charlie_report.into(),
		));
		let david_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &david, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// report 3rd offence
		assert_ok!(TesseractSigStorage::api_report_misbehavior(
			0, // setId is 0
			ALICE,
			david.into(),
			david_report.into(),
		));
		// 3 reported offences
		assert_eq!(3, TesseractSigStorage::commited_offences(ALICE).unwrap().0);
		let edward_report = keystore
			.sr25519_sign(time_primitives::KEY_TYPE, &edward, ALICE.as_ref())
			.unwrap()
			.unwrap();
		// cannot report offence because already past threshold so already
		// moved from reported to committed offences
		assert_noop!(
			TesseractSigStorage::api_report_misbehavior(
				0, // setId is 0
				ALICE,
				edward.into(),
				edward_report.into(),
			),
			Error::<Test>::OffenderAlreadyCommittedOffence
		);
	});
}
