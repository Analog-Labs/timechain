mod mock_client;

use std::{str::FromStr, time::Duration};

use futures::channel::oneshot;
use mock_client::MockBlock;
use std::thread;
use subxt::utils::H256;
pub use subxt_signer::sr25519::Keypair;
use subxt_signer::SecretUri;
use tc_subxt::timechain_client::IBlock;
use tc_subxt::worker::{SubxtWorker, Tx, MORTALITY};
use tokio::time::sleep;

use crate::mock_client::MockClient;
type TxSender = futures::channel::mpsc::UnboundedSender<(
	Tx,
	oneshot::Sender<<MockBlock as IBlock>::Extrinsic>,
)>;

#[tokio::test]
async fn test_transaction_flow() {
	let (client, tx_sender) = new_env().await;
	let (tx, rx) = oneshot::channel();
	tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	let hashes = wait_for_submission(&client, 1).await;
	assert_eq!(hashes.len(), 1);
	client.inc_block(Some(hashes[0])).await;
	let tx = rx.await.unwrap();
	assert_eq!(hashes[0], tx.hash);
}

#[tokio::test]
async fn test_transaction_mortality_outage_flow() {
	let (client, tx_sender) = new_env().await;
	let (tx, rx) = oneshot::channel();
	tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
	wait_for_submission(&client, 1).await;
	client.inc_empty_blocks(MORTALITY + 1).await;
	let hashes = wait_for_submission(&client, 2).await;
	assert_eq!(hashes.len(), 2);
	// one received in initial execution then again received from mortality outage
	assert_eq!(hashes[0], hashes[1]);
	client.inc_block(Some(hashes[0])).await;
	let tx = rx.await.unwrap();
	assert_eq!(hashes[0], tx.hash);
}

/////////////////////////////////////////////
/// Utils
////////////////////////////////////////////
async fn wait_for_submission(client: &MockClient, len: u8) -> Vec<H256> {
	loop {
		{
			let submitted = client.submitted_transactions().await;
			if submitted.len() >= len.into() {
				return submitted;
			}
		}
		tracing::info!("didnt find any submitted hash retrying");
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	}
}

/////////////////////////////////////////////
/// ENV setup
////////////////////////////////////////////
async fn new_env() -> (MockClient, TxSender) {
	env_logger::try_init().ok();
	let mock_client = MockClient::new();
	let uri = SecretUri::from_str("//Alice").unwrap();
	let keypair = Keypair::from_uri(&uri).unwrap();
	let worker = SubxtWorker::new(0, mock_client.clone(), keypair).await.unwrap();
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
	(mock_client, tx_sender)
}
