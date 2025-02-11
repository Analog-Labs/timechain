mod mock_client;

use std::collections::VecDeque;
use std::{str::FromStr, time::Duration};

use futures::channel::oneshot;
use mock_client::{compute_tx_hash, MockBlock, MockDb};
use std::thread;
use subxt::utils::H256;
pub use subxt_signer::sr25519::Keypair;
use subxt_signer::SecretUri;
use tc_subxt::timechain_client::{IBlock, ITransactionDbOps};
use tc_subxt::worker::{SubxtWorker, Tx, MORTALITY};
use tokio::time::sleep;

use crate::mock_client::MockClient;
type TxSender = futures::channel::mpsc::UnboundedSender<(
	Tx,
	oneshot::Sender<<MockBlock as IBlock>::Extrinsic>,
)>;

#[tokio::test]
async fn test_transaction_flow() {
	let env = new_env().await;
	let (tx, rx) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	let hashes = wait_for_submission(&env.client, 1).await;
	assert_eq!(hashes.len(), 1);
	env.client.inc_block_with_tx(hashes[0], true).await;
	let tx = rx.await.unwrap();
	assert_eq!(hashes[0], tx.hash);
}

// test tx failing and next tx being processed.
#[tokio::test]
async fn test_transaction_flow_with_error() {
	let env = new_env().await;
	let first_tx_hash = compute_tx_hash(0);
	env.client.failing_transactions(&first_tx_hash).await;

	let (tx, _) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	let hashes = wait_for_submission(&env.client, 1).await;
	assert_eq!(hashes.len(), 1);
	env.client.inc_block_with_tx(hashes[0], false).await;

	let (tx1, rx2) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 1 }, tx1)).unwrap();
	let hashes = wait_for_submission(&env.client, 2).await;
	assert_eq!(hashes.len(), 2);
	env.client.inc_block_with_tx(hashes[1], true).await;

	let tx = rx2.await.unwrap();
	assert_eq!(hashes[1], tx.hash);
}

#[tokio::test]
async fn test_transaction_mortality_outage_flow() {
	let env = new_env().await;
	let (tx, rx) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	wait_for_submission(&env.client, 1).await;
	env.client.inc_empty_blocks(MORTALITY + 1).await;
	let hashes = wait_for_submission(&env.client, 2).await;
	assert_eq!(hashes.len(), 2);
	// one received in initial execution then again received from mortality outage
	assert_eq!(hashes[0], hashes[1]);
	env.client.inc_block_with_tx(hashes[0], true).await;
	let tx = rx.await.unwrap();
	assert_eq!(hashes[0], tx.hash);
}

#[tokio::test]
#[ignore]
// not working tbf
async fn test_transaction_mortality_outage_flow_50() {
	let total_tasks: usize = 50;
	let env = new_env().await;
	let mut receivers = VecDeque::new();
	// init 100 transactions
	for _ in 0..total_tasks {
		let (tx, rx) = oneshot::channel();
		env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
		receivers.push_back(rx);
	}
	let hashes = wait_for_submission(&env.client, total_tasks).await;
	assert!(hashes.len() == total_tasks);
	env.client.inc_empty_blocks(MORTALITY / 2).await;
	tokio::time::sleep(Duration::from_secs(1)).await;
	env.client.inc_empty_blocks(MORTALITY / 2).await;
	tokio::time::sleep(Duration::from_secs(1)).await;
	env.client.inc_empty_blocks(1).await;
	let hashes = wait_for_submission(&env.client, total_tasks + total_tasks).await;
	assert_eq!(hashes[0], hashes[total_tasks + 1]);
	env.client.inc_block_with_tx(hashes[0], true).await;
	let rx = receivers.pop_back().unwrap();
	let tx = rx.await.unwrap();
	tracing::info!("hashes lenght: {}", hashes.len());
	assert_eq!(*hashes.last().unwrap(), tx.hash);
}

// test add tx to db
#[tokio::test]
async fn test_tc_subxt_db_ops() {
	let env = new_env().await;
	let (tx, rx) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	let hashes = wait_for_submission(&env.client, 1).await;
	assert!(hashes.len() == 1);
	// check db insertion
	let txs = env.db.load_pending_txs(0).unwrap();
	assert!(txs.len() == 1);
	assert_eq!(txs[0].hash, hashes[0]);
	env.client.inc_block_with_tx(hashes[0], true).await;
	let _ = rx.await.unwrap();
	let txs = env.db.load_pending_txs(0).unwrap();
	assert!(txs.len() == 0);
}

// Add test to check stream in case of errors.
#[tokio::test]
async fn test_finalized_stream_error_and_recovery() {
	// basic test flow
	let env = new_env().await;
	env.client.inc_empty_blocks(1).await;
	let (tx, rx) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	let hashes = wait_for_submission(&env.client, 1).await;
	assert_eq!(hashes.len(), 1);
	env.client.inc_block_with_tx(hashes[0], true).await;
	let tx = rx.await.unwrap();
	assert_eq!(hashes[0], tx.hash);

	// stream restart
	sleep(Duration::from_secs(1)).await;
	env.client.set_force_stream_error(true).await;
	sleep(Duration::from_millis(100)).await;
	env.client.set_force_stream_error(false).await;
	env.client.inc_empty_blocks(1).await;

	// adding new
	let (tx, rx) = oneshot::channel();
	env.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	let hashes = wait_for_submission(&env.client, 2).await;
	assert_eq!(hashes.len(), 2);
	env.client.inc_block_with_tx(hashes[1], true).await;
	let tx = rx.await.unwrap();
	assert_eq!(hashes[1], tx.hash);
}

/////////////////////////////////////////////
/// Utils
////////////////////////////////////////////
async fn wait_for_submission(client: &MockClient, len: usize) -> Vec<H256> {
	loop {
		{
			let submitted = client.submitted_transactions().await;
			tracing::info!("Submitted hashes length: {}", submitted.len());
			if submitted.len() >= len {
				return submitted;
			}
		}
		tracing::info!("didnt find any submitted hash retrying");
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	}
}

fn get_keypair() -> Keypair {
	let uri = SecretUri::from_str("//Alice").unwrap();
	Keypair::from_uri(&uri).unwrap()
}

/////////////////////////////////////////////
/// ENV setup
////////////////////////////////////////////

struct TestingEnv {
	client: MockClient,
	tx_sender: TxSender,
	db: MockDb,
}
async fn new_env() -> TestingEnv {
	env_logger::try_init().ok();
	let mock_client = MockClient::new();
	let keypair = get_keypair();
	let db = MockDb::default();
	let worker = SubxtWorker::new(0, mock_client.clone(), db.clone(), keypair).await.unwrap();
	let (tx_sender_tx, tx_sender_rx) = oneshot::channel::<TxSender>();

	// spawning into_sender in seperate thread because into_sender goes into busy waiting which blocks every other operation in testing.
	thread::spawn(move || {
		let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
		rt.block_on(async move {
			let tx_sender = worker.into_sender();
			tx_sender_tx.send(tx_sender).expect("Failed to send tx_sender via oneshot");
			// this prevents the thread to get closed and stops worker service
			futures::future::pending::<()>().await;
		});
	});

	let tx_sender = tx_sender_rx.await.expect("Failed to receive tx_sender");
	// wait for stream subscription happen in worker.into_sender;
	loop {
		{
			if *mock_client.subscription_counter.lock().await >= 2 {
				break;
			}
		}
		tracing::info!("Waiting on subscriptions..");
		sleep(Duration::from_millis(500)).await;
	}

	TestingEnv {
		client: mock_client,
		tx_sender,
		db,
	}
}
