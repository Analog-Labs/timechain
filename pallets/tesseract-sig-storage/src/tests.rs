use super::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok, error::BadOrigin, storage::bounded_vec::BoundedVec};
use frame_system::RawOrigin;
use sp_core::ConstU32;
use sp_keystore::Keystore;
use sp_runtime::AccountId32;
use time_primitives::{
	abstraction::{Function, ScheduleInput, TaskSchedule as ScheduleOut},
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
	let id = 1;
	let r: u8 = 123;
	let sig_data: [u8; 64] = [r; 64];
	let function = Function::EVMViewWithoutAbi {
		address: Default::default(),
		function_signature: Default::default(),
		input: Default::default(),
	};

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");

	new_test_ext().execute_with(|| {
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

		let tss_key = [0u8; 33];

		let alice_report_tss = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, tss_key.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::Signed(ALICE).into(),
			0,
			tss_key,
			alice_report_tss.into()
		));

		// insert schedule
		let input = ScheduleInput {
			network: Network::Ethereum,
			function,
			cycle: 12,
			hash: String::from("address"),
			frequency: 0,
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
	let id = 1;
	let r: u8 = 123;
	let sig_data: [u8; 64] = [r; 64];
	let function = Function::EVMViewWithoutAbi {
		address: Default::default(),
		function_signature: Default::default(),
		input: Default::default(),
	};

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");

	new_test_ext().execute_with(|| {
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

		let tss_key = [0u8; 33];

		let alice_report_tss = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, tss_key.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::Signed(ALICE).into(),
			0,
			tss_key,
			alice_report_tss.into()
		));

		// insert schedule
		let input = ScheduleInput {
			network: Network::Ethereum,
			function: function.clone(),
			cycle: 12,
			hash: String::from("address"),
			frequency: 0,
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
			owner: ALICE.clone(),
			network: Network::Ethereum,
			function,
			frequency: 0,
			cycle: 11,
			hash: String::from("address"),
			status: ScheduleStatus::Recurring,
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
	let id = 1;
	let r: u8 = 123;
	let sig_data: [u8; 64] = [r; 64];
	let function = Function::EVMViewWithoutAbi {
		address: Default::default(),
		function_signature: Default::default(),
		input: Default::default(),
	};

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");

	new_test_ext().execute_with(|| {
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

		let tss_key = [0u8; 33];

		let alice_report_tss = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, tss_key.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::Signed(ALICE).into(),
			0,
			tss_key,
			alice_report_tss.into()
		));

		// insert schedule
		let input = ScheduleInput {
			network: Network::Ethereum,
			function,
			cycle: 12,
			hash: String::from("address"),
			frequency: 1,
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
fn test_set_shard_online() {
	let shard_id = 0;
	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
	new_test_ext().execute_with(|| {
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

		assert!(!TesseractSigStorage::tss_shards(shard_id).unwrap().is_online());
		let tss_key = [0u8; 33];

		let alice_report_tss = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, tss_key.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::Signed(ALICE).into(),
			0,
			tss_key,
			alice_report_tss.into()
		));
		assert!(TesseractSigStorage::tss_shards(shard_id).unwrap().is_online());
	});
}

#[test]
fn test_force_set_shard_offline() {
	let shard_id = 0;

	let keystore = std::sync::Arc::new(sc_keystore::LocalKeystore::in_memory());
	let alice = keystore
		.sr25519_generate_new(time_primitives::TIME_KEY_TYPE, None)
		.expect("Creates authority key");
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

		let tss_key = [0u8; 33];

		let alice_report_tss = keystore
			.sr25519_sign(time_primitives::TIME_KEY_TYPE, &alice, tss_key.as_ref())
			.unwrap()
			.unwrap();

		assert_ok!(TesseractSigStorage::submit_tss_group_key(
			RawOrigin::Signed(ALICE).into(),
			0,
			tss_key,
			alice_report_tss.into()
		));
		assert_ok!(TesseractSigStorage::force_set_shard_offline(RawOrigin::Root.into(), shard_id));
		assert!(!TesseractSigStorage::tss_shards(shard_id).unwrap().is_online());
	});
}
