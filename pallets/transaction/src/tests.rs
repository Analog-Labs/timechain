use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use sp_core::{H256, H512};

fn sample_transaction1() -> Transaction {
	Transaction {
		header: TxHeader { event_hash: H256::random(), previous_hash: H256::random() },
		body: TxBody {
			event_name: br#"e{"Test event"}"#.to_vec(),
			category: br#"e{"Test category"}"#.to_vec(),
			subject: H256::random(),
			object: H256::random(),
			time_attributes: 0,
		},
		footer: TxFooter { broadcaster_signature: H512::random() },
	}
}

#[test]
fn test_create_new_event() {
	new_test_ext().execute_with(|| {
		let transaction = sample_transaction1();
		assert_ok!(TransactionModule::create_event(Origin::signed(1), transaction));
	});
}

#[test]
fn test_create_event_that_already_exists() {
	new_test_ext().execute_with(|| {
		let transaction = sample_transaction1();
		let transaction_copy = transaction.clone();

		// Event Created
		assert_ok!(TransactionModule::create_event(Origin::signed(1), transaction));

		// Evet already exists
		assert_noop!(
			TransactionModule::create_event(Origin::signed(1), transaction_copy),
			Error::<Test>::EventAlreadyExists
		);
	});
}

#[test]
fn test_delete_event_by_owner() {
	new_test_ext().execute_with(|| {
		let transaction = sample_transaction1();
		let transaction_copy = transaction.clone();

		// Create an event first
		TransactionModule::create_event(Origin::signed(1), transaction).unwrap();

		// Delete the event
		assert_ok!(TransactionModule::delete_event(Origin::signed(1), transaction_copy));
	});
}

#[test]
fn test_delete_event_that_doese_not_exist() {
	new_test_ext().execute_with(|| {
		let transaction = sample_transaction1();

		// Try to delete an event that doesn't exist
		assert_noop!(
			TransactionModule::delete_event(Origin::signed(1), transaction),
			Error::<Test>::NoSuchEvent
		);
	});
}

#[test]
fn test_delete_event_created_by_some_other_owner() {
	new_test_ext().execute_with(|| {
		let transaction = sample_transaction1();
		let transaction_copy = transaction.clone();
		let owner1 = 1;
		let owner2 = 2;

		// Create an event first
		TransactionModule::create_event(Origin::signed(owner1), transaction).unwrap();

		// Try to delete an event that doesn't exist
		assert_noop!(
			TransactionModule::delete_event(Origin::signed(owner2), transaction_copy),
			Error::<Test>::NotEventOwner
		);
	});
}
