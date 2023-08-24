use crate::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_core::offchain::testing::{TestOffchainExt, TestTransactionPoolExt};
use sp_core::offchain::{OffchainDbExt, OffchainWorkerExt, TransactionPoolExt};
use sp_core::Decode;
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use time_primitives::{OcwPayload, ShardId, TssPublicKey, TIME_KEY_TYPE};

pub const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
const SHARD_ID: ShardId = 42;
const TSS_PUBLIC_KEY: TssPublicKey = [42; 33];
const PAYLOAD: OcwPayload = OcwPayload::SubmitTssPublicKey {
	shard_id: SHARD_ID,
	public_key: TSS_PUBLIC_KEY,
};

#[test]
fn test_ocw_read_message() {
	env_logger::try_init().ok();
	let mut ext = new_test_ext();
	let storage = ext.offchain_db();
	time_primitives::write_message_with_prefix(storage.clone(), &[], &PAYLOAD);
	time_primitives::write_message_with_prefix(storage.clone(), &[], &PAYLOAD);
	let (offchain, offchain_state) = TestOffchainExt::with_offchain_db(storage);
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	log::info!("{:?}", offchain_state);
	ext.execute_with(|| {
		assert_eq!(Ocw::read_message().as_ref(), Some(&PAYLOAD));
		assert_eq!(Ocw::read_message().as_ref(), Some(&PAYLOAD));
		assert_eq!(Ocw::read_message(), None);
	});
}

#[test]
fn test_ocw_submit_tx() {
	env_logger::try_init().ok();
	let mut ext = new_test_ext();
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	let keystore = MemoryKeystore::new();
	let collector = keystore.sr25519_generate_new(TIME_KEY_TYPE, Some(PHRASE)).unwrap();
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(KeystoreExt::new(keystore));
	ext.execute_with(|| {
		Ocw::submit_tx(PAYLOAD);
		log::info!("{:?}", offchain_state);
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = UncheckedExtrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature.unwrap().0, collector.into());
		assert_eq!(
			tx.function,
			RuntimeCall::Ocw(Call::submit_tss_public_key {
				shard_id: SHARD_ID,
				public_key: TSS_PUBLIC_KEY
			})
		);
	});
}

#[test]
fn test_submit_public_key() {
	env_logger::try_init().ok();
	let mut ext = new_test_ext();
	let keystore = MemoryKeystore::new();
	let collector = keystore.sr25519_generate_new(TIME_KEY_TYPE, Some(PHRASE)).unwrap();
	ext.execute_with(|| {
		assert_ok!(Ocw::submit_tss_public_key(
			RawOrigin::Signed(collector.into()).into(),
			SHARD_ID,
			TSS_PUBLIC_KEY
		));
		assert_eq!(SHARD_PUBLIC_KEYS.lock().unwrap().get(&SHARD_ID), Some(&TSS_PUBLIC_KEY));
	});
}

#[test]
fn test_submit_public_key_not_collector() {
	env_logger::try_init().ok();
	let mut ext = new_test_ext();
	ext.execute_with(|| {
		assert_noop!(
			Ocw::submit_tss_public_key(
				RawOrigin::Signed([42; 32].into()).into(),
				SHARD_ID,
				TSS_PUBLIC_KEY
			),
			Error::<Test>::NotSignedByCollector
		);
	});
}
