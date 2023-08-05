use super::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use frame_system::RawOrigin;
use sp_keystore::Keystore;
use time_primitives::{
	abstraction::{Function, ScheduleInput, TaskSchedule as ScheduleOut},
	Network, TimeId,
};

#[test]
fn test_signature_storage() {
	let _id = 1;
	let r: u8 = 123;
	let _sig_data: [u8; 64] = [r; 64];
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

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
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
			period: 0,
			start: 1,
		};

		assert_ok!(TaskSchedule::create_task(RawOrigin::Signed(ALICE).into(), input.clone()));
	});
}

#[test]
fn test_signature_and_decrement_schedule_storage() {
	let _id = 1;
	let r: u8 = 123;
	let _sig_data: [u8; 64] = [r; 64];
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

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
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
			start: 1,
			period: 0,
		};

		assert_ok!(TaskSchedule::create_task(RawOrigin::Signed(ALICE).into(), input.clone()));

		let output = ScheduleOut {
			owner: ALICE.clone(),
			network: Network::Ethereum,
			function,
			cycle: 11,
			hash: String::from("address"),
			start: 1,
			period: 0,
		};

		let scheduled_task = TaskSchedule::get_task(1_u64);
		assert_eq!(scheduled_task, Some(output));
	});
}

#[test]
fn test_duplicate_signature() {
	let _id = 1;
	let r: u8 = 123;
	let _sig_data: [u8; 64] = [r; 64];
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
		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
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
			start: 0,
			period: 1,
		};

		assert_ok!(TaskSchedule::create_task(RawOrigin::Signed(ALICE).into(), input.clone()));
	});
}

#[test]
fn test_register_shard_fails_if_duplicate_members() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				vec![ALICE, BOB, BOB],
				Network::Ethereum,
			),
			Error::<Test>::PublicKeyAlreadyRegistered
		);
	});
}

#[test]
fn test_register_shard_fails_if_member_len_not_supported() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			TesseractSigStorage::register_shard(
				RawOrigin::Root.into(),
				vec![ALICE, BOB, CHARLIE, DJANGO],
				Network::Ethereum,
			),
			Error::<Test>::InvalidNumberOfShardMembers
		);
	});
}

#[test]
/// Currently supported sizes are 3, 5, 10
fn test_register_shard_works_for_supported_member_lengths() {
	new_test_ext().execute_with(|| {
		let tmp_time_id = TimeId::new([5u8; 32]);
		let mut members = vec![ALICE, BOB, CHARLIE];
		// supports 3
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			members.clone(),
			Network::Ethereum,
		));

		// supports 5
		members.push(DJANGO);
		members.push(tmp_time_id);
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			members.clone(),
			Network::Ethereum,
		));

		// supports 10
		for i in 6..=8 {
			let tmp_time_id = TimeId::new([i as u8; 32]);

			members.push(tmp_time_id);
		}

		for i in 9..=10 {
			let tmp_time_id = TimeId::new([i as u8; 32]);

			members.push(tmp_time_id);
		}
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			members,
			Network::Ethereum
		));
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
		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
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
		// assert_noop!(
		// 	TesseractSigStorage::force_set_shard_offline(
		// 		RawOrigin::Signed(VALIDATOR_1).into(),
		// 		shard_id
		// 	),
		// 	BadOrigin
		// );

		// assert_noop!(
		// 	TesseractSigStorage::force_set_shard_offline(RawOrigin::Root.into(), shard_id),
		// 	Error::<Test>::InvalidShardId
		// );

		//register shard
		assert_ok!(TesseractSigStorage::register_shard(
			RawOrigin::Root.into(),
			vec![alice.into(), BOB, CHARLIE],
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
	});
}
