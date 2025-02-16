use crate::mock::TestingEnv;
use anyhow::Result;
use std::time::Duration;
use tc_subxt::timechain_client::ITransactionDbOps;
use tc_subxt::worker::MORTALITY;
use tokio::time::sleep;

mod mock;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_transaction_flow() -> Result<()> {
	let env = TestingEnv::new().await;
	let rx = env.submit_tx().await;
	let nonce = env.submission().await;
	assert_eq!(nonce, 0);
	env.execute_tx(nonce, true).await;
	env.make_block().await;
	let tx = rx.await?;
	assert_eq!(tx.nonce, 0);
	assert!(tx.success);
	Ok(())
}

// test tx failing and next tx being processed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_transaction_flow_with_error() -> Result<()> {
	let env = TestingEnv::new().await;
	let rx = env.submit_tx().await;
	let nonce = env.submission().await;
	assert_eq!(nonce, 0);
	env.execute_tx(nonce, false).await;
	env.make_block().await;
	let tx = rx.await?;
	assert_eq!(tx.nonce, 0);
	assert!(!tx.success);
	Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_transaction_mortality_outage_flow() -> Result<()> {
	let env = TestingEnv::new().await;
	let rx = env.submit_tx().await;
	let nonce = env.submission().await;
	assert_eq!(nonce, 0);
	for _ in 0..(MORTALITY + 1) {
		env.make_block().await;
	}
	let nonce = env.submission().await;
	assert_eq!(nonce, 0);
	env.execute_tx(nonce, true).await;
	env.make_block().await;
	let tx = rx.await?;
	assert_eq!(tx.nonce, 0);
	assert!(tx.success);
	Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_transaction_mortality_outage_flow_20() -> Result<()> {
	let num_txs = 20;
	let env = TestingEnv::new().await;
	let mut rxs = vec![];
	for _ in 0..num_txs {
		let rx = env.submit_tx().await;
		rxs.push(rx);
	}
	for _ in 0..num_txs {
		env.submission().await;
	}
	for _ in 0..(MORTALITY + 1) {
		env.make_block().await;
	}
	for _ in 0..num_txs {
		let nonce = env.submission().await;
		env.execute_tx(nonce, true).await;
	}
	env.make_block().await;
	for rx in rxs {
		let tx = rx.await?;
		assert!(tx.success);
	}
	Ok(())
}

// test add tx to db
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_tc_subxt_db_ops() -> Result<()> {
	let env = TestingEnv::new().await;
	let rx = env.submit_tx().await;
	let nonce = env.submission().await;
	// check db insertion
	let txs = env.db.load_pending_txs(0).unwrap();
	assert!(txs.len() == 1);
	env.execute_tx(nonce, true).await;
	env.make_block().await;
	let _ = rx.await.unwrap();
	let txs = env.db.load_pending_txs(0).unwrap();
	assert!(txs.is_empty());
	Ok(())
}

// Add test to check stream in case of errors.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_finalized_stream_error_and_recovery() -> Result<()> {
	let env = TestingEnv::new().await;
	let rx = env.submit_tx().await;
	let nonce = env.submission().await;
	assert_eq!(nonce, 0);
	env.execute_tx(nonce, true).await;
	env.make_block().await;
	let tx = rx.await?;
	assert_eq!(tx.nonce, 0);
	assert!(tx.success);

	// stream restart
	sleep(Duration::from_secs(1)).await;
	env.set_force_stream_error(true).await;
	sleep(Duration::from_millis(100)).await;
	env.set_force_stream_error(false).await;
	env.make_block().await;

	// adding new
	let rx = env.submit_tx().await;
	let nonce = env.submission().await;
	assert_eq!(nonce, 1);
	env.execute_tx(nonce, true).await;
	env.make_block().await;
	let tx = rx.await?;
	assert_eq!(tx.nonce, 1);
	assert!(tx.success);
	Ok(())
}
