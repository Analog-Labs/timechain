use super::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok, error::BadOrigin, storage::bounded_vec::BoundedVec};
use frame_system::RawOrigin;
use sp_core::ConstU32;
use sp_keystore::Keystore;
use time_primitives::{
	abstraction::{ObjectId, ScheduleInput, TaskSchedule as ScheduleOut, Validity},
	sharding::Network,
	ScheduleStatus, TimeId,
};

#[test]
fn test_register_chronicle() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			TesseractSigStorage::register_chronicle(
				RawOrigin::Signed(INVALID_VALIDATOR).into(),
				ALICE,
			),
			Error::<Test>::OnlyValidatorCanRegisterChronicle
		);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		let expected_data = BoundedVec::<TimeId, ConstU32<3>>::truncate_from(vec![ALICE]);
		assert_eq!(ValidatorToChronicle::<Test>::get(VALIDATOR_1), Some(expected_data));
		frame_system::Pallet::<Test>::assert_last_event(
			Event::<Test>::ChronicleRegistered(ALICE, VALIDATOR_1).into(),
		);

		assert_noop!(
			TesseractSigStorage::register_chronicle(RawOrigin::Signed(VALIDATOR_2).into(), ALICE,),
			Error::<Test>::ChronicleAlreadyRegistered
		);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			BOB,
		),);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			CHARLIE,
		),);

		assert_noop!(
			TesseractSigStorage::register_chronicle(RawOrigin::Signed(VALIDATOR_1).into(), DJANGO,),
			Error::<Test>::ChronicleSetIsFull
		);
	});
}

#[test]
fn test_signature_storage() {
	let r: u8 = 123;
	let sig_data: [u8; 64] = [r; 64];

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");

	new_test_ext().execute_with(|| {
		let id: u64 = 1;
		let task_id = ObjectId(id);
		let block_number = 10;
		System::set_block_number(block_number);
		let shard_id = 0;

		// check the init status before any signature is stored
		assert_eq!(Pallet::<Test>::last_committed_chronicle(Into::<TimeId>::into(alice)), 0);
		assert_eq!(Pallet::<Test>::last_committed_shard(shard_id), 0);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			alice.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
			Some(0),
			Network::Ethereum,
		),);

		// insert schedule
		let input = ScheduleInput {
			task_id,
			shard_id,
			cycle: 12,
			validity: Validity::Seconds(1000),
			hash: String::from("address"),
			frequency: 0,
			status: time_primitives::ScheduleStatus::Initiated,
		};

		let alice_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, sig_data.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(ALICE).into(), input.clone()));

		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(ALICE).into(),
			alice_report.into(),
			sig_data,
			id,
			input.cycle
		));

		assert!(TesseractSigStorage::signature_storage(id, input.cycle).is_some());
		assert_eq!(
			Pallet::<Test>::last_committed_chronicle(Into::<TimeId>::into(alice)),
			block_number
		);
		assert_eq!(Pallet::<Test>::last_committed_shard(shard_id), block_number);
	});
}

#[test]
fn test_signature_and_decrement_schedule_storage() {
	let r: u8 = 123;
	let sig_data: [u8; 64] = [r; 64];

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");

	new_test_ext().execute_with(|| {
		let id: u64 = 1;
		let task_id = ObjectId(id);
		let block_number = 10;
		System::set_block_number(block_number);
		let shard_id = 0;

		// check the init status before any signature is stored
		assert_eq!(Pallet::<Test>::last_committed_chronicle(Into::<TimeId>::into(alice)), 0);
		assert_eq!(Pallet::<Test>::last_committed_shard(shard_id), 0);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			alice.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
			Some(0),
			Network::Ethereum,
		),);

		// insert schedule
		let input = ScheduleInput {
			task_id,
			shard_id,
			cycle: 12,
			validity: Validity::Seconds(1000),
			hash: String::from("address"),
			frequency: 0,
			status: time_primitives::ScheduleStatus::Initiated,
		};

		let alice_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, sig_data.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(ALICE).into(), input.clone()));

		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(ALICE).into(),
			alice_report.into(),
			sig_data,
			id,
			input.cycle
		));

		assert!(TesseractSigStorage::signature_storage(id, input.cycle).is_some());

		let output = ScheduleOut {
			task_id: ObjectId(1),
			owner: ALICE.clone(),
			shard_id: 0,
			frequency: 0,
			start_execution_block: 0,
			executable_since: block_number,
			cycle: 11,
			validity: Validity::Seconds(1000),
			hash: String::from("address"),
			status: ScheduleStatus::Updated,
		};

		let scheduled_task = TaskSchedule::get_task_schedule(1_u64);
		assert_eq!(scheduled_task, Some(output));

		assert_eq!(
			Pallet::<Test>::last_committed_chronicle(Into::<TimeId>::into(alice)),
			block_number
		);
		assert_eq!(Pallet::<Test>::last_committed_shard(shard_id), block_number);
	});
}

#[test]
fn test_duplicate_signature() {
	let r: u8 = 123;
	let sig_data: [u8; 64] = [r; 64];

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");

	new_test_ext().execute_with(|| {
		let id: u64 = 1;
		let task_id = ObjectId(id);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			alice.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
			Some(0),
			Network::Ethereum,
		),);

		// insert schedule
		let input = ScheduleInput {
			task_id,
			shard_id: 0,
			cycle: 12,
			validity: Validity::Seconds(1000),
			hash: String::from("address"),
			frequency: 1,
			status: time_primitives::ScheduleStatus::Initiated,
		};

		assert_ok!(TaskSchedule::insert_schedule(RawOrigin::Signed(ALICE).into(), input.clone()));

		let alice_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, sig_data.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TesseractSigStorage::store_signature(
			RawOrigin::Signed(ALICE).into(),
			alice_report.clone().into(),
			sig_data,
			id,
			input.cycle
		));

		assert_noop!(
			TesseractSigStorage::store_signature(
				RawOrigin::Signed(ALICE).into(),
				alice_report.into(),
				sig_data,
				id,
				input.cycle
			),
			Error::<Test>::DuplicateSignature
		);
	});
}

#[test]
fn test_register_shard_fails_if_duplicate_members() {
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				vec![ALICE, BOB, BOB],
				Some(0),
				Network::Ethereum,
			),
			Error::<Test>::DuplicateShardMembersNotAllowed
		);
	});
}

#[test]
fn test_register_shard_fails_if_member_len_not_supported() {
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_4).into(),
			DJANGO,
		),);
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				vec![ALICE, BOB, CHARLIE, DJANGO],
				Some(0),
				Network::Ethereum,
			),
			Error::<Test>::UnsupportedMembershipSize
		);
	});
}

#[test]
/// Currently supported sizes are 3, 5, 10
fn test_register_shard_works_for_supported_member_lengths() {
	new_test_ext().execute_with(|| {
		let tmp_time_id = TimeId::new([5u8; 32]);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			CHARLIE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			DJANGO,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			tmp_time_id.clone(),
		),);
		let mut members = vec![ALICE, BOB, CHARLIE];
		// supports 3
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			members.clone(),
			Some(1),
			Network::Ethereum,
		));

		// supports 5
		members.push(DJANGO);
		members.push(tmp_time_id);
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			members.clone(),
			Some(1),
			Network::Ethereum,
		));

		// supports 10
		for i in 6..=8 {
			let tmp_time_id = TimeId::new([i as u8; 32]);
			assert_ok!(TesseractSigStorage::register_chronicle(
				RawOrigin::Signed(VALIDATOR_3).into(),
				tmp_time_id.clone(),
			),);
			members.push(tmp_time_id);
		}

		for i in 9..=10 {
			let tmp_time_id = TimeId::new([i as u8; 32]);
			assert_ok!(TesseractSigStorage::register_chronicle(
				RawOrigin::Signed(VALIDATOR_4).into(),
				tmp_time_id.clone(),
			),);
			members.push(tmp_time_id);
		}
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			members,
			Some(1),
			Network::Ethereum
		));
	});
}

#[test]
fn test_register_shard_fails_if_collector_index_invalid() {
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				vec![ALICE, BOB, CHARLIE],
				Some(4),
				Network::Ethereum,
			),
			Error::<Test>::CollectorIndexBeyondMemberLen
		);
	});
}

#[test]
fn test_report_misbehavior_increments_report_count() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let bob = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			alice.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			bob.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), bob.into(), CHARLIE],
			Some(0),
			Network::Ethereum,
		));

		// report 1st offence

		let alice_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			CHARLIE,
			alice_report.clone().into(),
		));
		// report 2nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			CHARLIE,
			alice_report.into(),
		));
		// 2 reported offences
		assert_eq!(2, TesseractSigStorage::commited_offences(CHARLIE).unwrap().0);
	});
}

#[test]
fn test_report_misbehavior_updates_reporters() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let bob = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			alice.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			bob.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), bob.into(), CHARLIE],
			Some(0),
			Network::Ethereum,
		));

		// report 1st offence

		let alice_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			CHARLIE,
			alice_report.clone().into(),
		));
		// report 2nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			CHARLIE,
			alice_report.into(),
		));
		// alice is only reporter
		assert!(TesseractSigStorage::commited_offences(CHARLIE)
			.unwrap()
			.1
			.contains(&alice.into()));
	});
}

#[test]
/// Moves offences to committed such that reported offences is emptied
fn test_report_misbehavior_moves_offences_to_committed() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let bob = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			alice.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			bob.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);

		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), bob.into(), CHARLIE],
			Some(0),
			Network::Ethereum,
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let alice_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, CHARLIE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			CHARLIE,
			alice_report.clone().into(),
		));
		// report 2nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			CHARLIE,
			alice_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(CHARLIE).is_some());
		// remove reported_offences from storage once moved to commited_offences
		assert!(TesseractSigStorage::reported_offences(CHARLIE).is_none());
	});
}

#[test]
fn test_report_misbehavior_for_group_len_5() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let charlie = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let david = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let edward = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			charlie.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			david.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			edward.into(),
		),);
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![ALICE, BOB, charlie.into(), david.into(), edward.into()],
			Some(2),
			Network::Ethereum,
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let charlie_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &charlie, ALICE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			charlie_report.clone().into(),
		));
		// report 2nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			charlie_report.clone().into(),
		));
		// report 3nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			charlie_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(ALICE).is_some());
	});
}

#[test]
fn test_report_misbehavior_for_group_len_10() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let edward = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let frank = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let greg = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let hank = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let indigo = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let jared = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			CHARLIE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			DJANGO,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			edward.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			frank.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			greg.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			hank.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_4).into(),
			indigo.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_4).into(),
			jared.into(),
		),);
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
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
			Some(4),
			Network::Ethereum,
		));
		// To report offence, need to sign the public key

		// report 1st offence

		let edward_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &edward, ALICE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			edward_report.clone().into(),
		));
		// report 2nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			edward_report.clone().into(),
		));
		// report 3nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			edward_report.clone().into(),
		));
		// report 4th offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			edward_report.clone().into(),
		));
		// report 5th offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			edward_report.into(),
		));
		assert!(TesseractSigStorage::commited_offences(ALICE).is_some());
	});
}

#[test]
/// Once moved to committed, reports only update committed offences
/// and do not update reported offences (remains empty)
fn can_report_offence_if_already_committed_offender() {
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	// test the thresholds
	let bob = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let charlie = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let david = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	let edward = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			bob.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			charlie.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			david.into(),
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			edward.into(),
		),);
		// register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![ALICE, bob.into(), charlie.into(), david.into(), edward.into()],
			Some(1),
			Network::Ethereum,
		));

		// report 1st offence
		let bob_report = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &bob, ALICE.as_ref())
			.unwrap()
			.unwrap();
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			bob_report.clone().into(),
		));
		// report 2nd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			bob_report.clone().into(),
		));
		// report 3rd offence
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			bob_report.clone().into(),
		));
		// 3 reported offences
		assert_eq!(3, TesseractSigStorage::commited_offences(ALICE).unwrap().0);
		// can report offence but only updates committed offences
		// such that reported offences stays empty
		assert_ok!(TesseractSigStorage::report_misbehavior(
			RawOrigin::Signed(ALICE).into(),
			0, // setId is 0
			ALICE,
			bob_report.into(),
		));
		// 4 reported offences in committed offences
		assert_eq!(4, TesseractSigStorage::commited_offences(ALICE).unwrap().0);
		// reported offences storage item remains empty
		assert!(TesseractSigStorage::reported_offences(ALICE).is_none());
	});
}

#[test]
fn test_set_shard_online() {
	let shard_id = 0;
	let tss_key: [u8; 33] = [0; 33];
	new_test_ext().execute_with(|| {
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![ALICE, BOB, CHARLIE],
			Some(0),
			Network::Ethereum,
		),);

		assert!(!TesseractSigStorage::tss_shards(shard_id).unwrap().is_online());
		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::None.into(),
			shard_id,
			tss_key
		));
		assert!(TesseractSigStorage::tss_shards(shard_id).unwrap().is_online());
	});
}

#[test]
fn test_force_set_shard_offline() {
	let shard_id = 0;
	let tss_key: [u8; 33] = [0; 33];
	new_test_ext().execute_with(|| {
		assert_noop!(
			TesseractSigStorage::force_set_shard_offline(
				RawOrigin::Signed(VALIDATOR_1).into(),
				shard_id
			),
			BadOrigin
		);

		assert_noop!(
			TesseractSigStorage::force_set_shard_offline(RawOrigin::Root.into(), shard_id),
			Error::<Test>::ShardIsNotRegistered
		);

		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_1).into(),
			ALICE,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_2).into(),
			BOB,
		),);
		assert_ok!(TesseractSigStorage::register_chronicle(
			RawOrigin::Signed(VALIDATOR_3).into(),
			CHARLIE,
		),);

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![ALICE, BOB, CHARLIE],
			Some(0),
			Network::Ethereum,
		),);

		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::None.into(),
			shard_id,
			tss_key
		));
		assert_ok!(TesseractSigStorage::force_set_shard_offline(RawOrigin::Root.into(), shard_id));
		assert!(!TesseractSigStorage::tss_shards(shard_id).unwrap().is_online());
	});
}
