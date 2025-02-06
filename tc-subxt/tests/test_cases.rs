mod mock_client;

use std::{str::FromStr, time::Duration};

use futures::channel::oneshot;
use mock_client::MockBlock;
use std::thread;
use subxt::utils::H256;
pub use subxt_signer::sr25519::Keypair;
use subxt_signer::SecretUri;
use tc_subxt::timechain_client::IBlock;
use tc_subxt::worker::{SubxtWorker, Tx};
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
	let tx_hash = wait_for_submission(&client).await;
	client.inc_block(Some(tx_hash)).await;
	let tx = rx.await.unwrap();
	assert_eq!(tx_hash, tx.hash);
}

/////////////////////////////////////////////
/// Utils
////////////////////////////////////////////
async fn wait_for_submission(client: &MockClient) -> H256 {
	loop {
		{
			let submitted = client.submitted_transactions().await;
			if !submitted.is_empty() {
				return submitted[0];
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
