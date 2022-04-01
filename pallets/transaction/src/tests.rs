use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use sp_core::{H256, H512};

#[test]
fn test_publish_valid_event() {
	new_test_ext().execute_with(|| {
		let tx = sample_transaction1();

		assert_ok!(TransactionPallet::publish_event(
			Origin::signed(1),
			tx.clone().header.source_chain_id,
			tx.clone().header.destination_chain_id,
			tx.clone().header.tessaract_hash,
			tx.clone().body.event_publisher,
			tx.clone().body.event_name,
			tx.clone().body.event_category,
			tx.clone().body.event_data
		));
	});
}

#[test]
fn test_published_event_stored_in_db() {
	new_test_ext().execute_with(|| {
		let tx = sample_transaction1();

		TransactionPallet::publish_event(
			Origin::signed(1),
			tx.clone().header.source_chain_id,
			tx.clone().header.destination_chain_id,
			tx.clone().header.tessaract_hash,
			tx.clone().body.event_publisher,
			tx.clone().body.event_name,
			tx.clone().body.event_category,
			tx.clone().body.event_data,
		);

		let account = vec![1, 0, 0, 0, 0, 0, 0, 0];
		let acc_slice = &account[..];
		let account_hash = encode_account(&acc_slice);

		// Get Transaction hash from database
		let tx_hash = TransactionPallet::get_tx_hash_by_account_hash(&account_hash);

		// Use previously fetched transaction hash to get the actual tx from database.
		let tx2 = TransactionPallet::get_tx_by_hash(&tx_hash).transaction;

		assert_eq!(tx2.body.event_name, tx.body.event_name);
	});
}

#[test]
fn test_publish_event_validate_previous_hash_fields() {
	new_test_ext().execute_with(|| {
		let tx = sample_transaction1();

		assert_ok!(TransactionPallet::publish_event(
			Origin::signed(1),
			tx.clone().header.source_chain_id,
			tx.clone().header.destination_chain_id,
			tx.clone().header.tessaract_hash,
			tx.clone().body.event_publisher,
			tx.clone().body.event_name,
			tx.clone().body.event_category,
			tx.clone().body.event_data,
		));

		// Get Transaction hash from database
		let tx_hash = TransactionPallet::get_tx_hash_by_publisher_hash(&tx.body.event_publisher);
		// Use previously fetched transaction hash to get the actual tx from database.
		let tx2 = TransactionPallet::get_tx_by_hash(&tx_hash).transaction;

		// Check that previous_tx_hash fields are filled with zeros,
		// as there is only one published event
		assert_eq!(tx2.header.previous_tx_hash_by_publisher, H256::zero());
		assert_eq!(tx2.header.previous_tx_hash_by_account, H256::zero());
		assert_eq!(tx2.header.previous_tx_hash_by_tessaract, H256::zero());
	});
}

#[test]
fn test_publish_multiple_events() {
	new_test_ext().execute_with(|| {
		let tx = sample_transaction1();

		assert_ok!(TransactionPallet::publish_event(
			Origin::signed(1),
			tx.clone().header.destination_chain_id,
			tx.clone().header.destination_chain_id,
			tx.clone().header.tessaract_hash,
			tx.clone().body.event_publisher,
			br#"e{"Test event1"}"#.to_vec(),
			tx.clone().body.event_category,
			tx.clone().body.event_data
		));

		assert_ok!(TransactionPallet::publish_event(
			Origin::signed(1),
			tx.clone().header.destination_chain_id,
			tx.clone().header.destination_chain_id,
			tx.clone().header.tessaract_hash,
			tx.clone().body.event_publisher,
			br#"e{"Test event2"}"#.to_vec(),
			tx.clone().body.event_category,
			tx.clone().body.event_data
		));

		// Get recent transaction hash (by publisher) from database
		let tx2_hash = TransactionPallet::get_tx_hash_by_publisher_hash(&tx.body.event_publisher);
		assert_ne!(tx2_hash, H256::zero());

		let tx2_from_db = TransactionPallet::get_tx_by_hash(&tx2_hash).transaction;

		assert_eq!(tx2_from_db.body.event_name, br#"e{"Test event2"}"#.to_vec());

		// Recent transaction is linked with the previous one, by account_hash.
		assert_ne!(tx2_from_db.header.previous_tx_hash_by_account, H256::zero());

		let tx1_from_db =
			TransactionPallet::get_tx_by_hash(&tx2_from_db.header.previous_tx_hash_by_account)
				.transaction;

		assert_eq!(tx1_from_db.body.event_name, br#"e{"Test event1"}"#.to_vec());

		// No more previous transactions
		assert_eq!(tx1_from_db.header.previous_tx_hash_by_account, H256::zero());
	});
}

// Helper functions

fn sample_transaction1() -> Transaction {
	Transaction {
		header: TxHeader {
			source_chain_id: 1,
			destination_chain_id: 1,
			event_hash: H256::random(),
			tessaract_hash: H256::random(),
			previous_tx_hash_by_publisher: H256::random(),
			previous_tx_hash_by_account: H256::random(),
			previous_tx_hash_by_tessaract: H256::random(),
		},
		body: TxBody {
			event_publisher: H256::random(),
			event_name: br#"e{"Test event"}"#.to_vec(),
			event_category: br#"e{"Test category"}"#.to_vec(),
			event_data: br#"e{""}"#.to_vec(),
			timestamp: 0,
		},
		footer: TxFooter { threshold_signature: H512::random() },
	}
}

fn encode_account(account: &[u8]) -> H256 {
	let acc_encoded = &account[..];
	let mut acc_vec = acc_encoded.to_vec();
	let account_hash: H256;

	if acc_vec.len() < 32 {
		let mut acc_vec_fix_size = vec![];
		acc_vec_fix_size.resize(32 - acc_vec.len(), 0);
		acc_vec_fix_size.append(&mut acc_vec);
		account_hash = H256::from_slice(&acc_vec_fix_size[..]);
	} else {
		account_hash = H256::from_slice(&acc_encoded[..]);
	}

	account_hash
}
