use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use futures::stream::{self, BoxStream};
use futures::{SinkExt, Stream, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};
use subxt::{tx::Payload as TxPayload, utils::H256};
use subxt_signer::{sr25519::Keypair, SecretUri};
use tc_subxt::timechain_client::{
	BlockId, IBlock, IExtrinsic, ITimechainClient, ITransactionDbOps, ITransactionSubmitter,
};
use tc_subxt::worker::{SubxtWorker, Tx, TxData};
use tc_subxt::ExtrinsicParams;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;

type TxSender = futures::channel::mpsc::UnboundedSender<(
	Tx,
	oneshot::Sender<<MockBlock as IBlock>::Extrinsic>,
)>;

pub struct TestingEnv {
	client: MockClient,
	tx_sender: TxSender,
	pub db: MockDb,
}

impl TestingEnv {
	pub async fn new() -> Self {
		env_logger::try_init().ok();
		let client = MockClient::new();
		let uri = SecretUri::from_str("//Alice").unwrap();
		let keypair = Keypair::from_uri(&uri).unwrap();
		let db = MockDb::default();
		let worker = SubxtWorker::new(0, client.clone(), db.clone(), keypair).await.unwrap();
		let tx_sender = worker.into_sender();
		Self { client, tx_sender, db }
	}

	pub async fn submit_tx(&self) -> oneshot::Receiver<MockExtrinsic> {
		let (tx, rx) = oneshot::channel();
		self.tx_sender.unbounded_send((Tx::Ready { shard_id: 0 }, tx)).unwrap();
		rx
	}
}

impl std::ops::Deref for TestingEnv {
	type Target = MockClient;

	fn deref(&self) -> &Self::Target {
		&self.client
	}
}

#[derive(Clone)]
pub struct MockClient {
	best_sender: broadcast::Sender<MockBlock>,
	finalized_sender: broadcast::Sender<MockBlock>,
	pool_sender: mpsc::Sender<u64>,
	pool: Arc<Mutex<mpsc::Receiver<u64>>>,
	executed: Arc<Mutex<Vec<MockExtrinsic>>>,
	best_block: Arc<Mutex<BlockId>>,
	subscription_counter: Arc<Mutex<u8>>,
	force_stream_error: Arc<Mutex<bool>>,
}

impl Default for MockClient {
	fn default() -> Self {
		Self::new()
	}
}

impl MockClient {
	pub fn new() -> Self {
		let (finalized_sender, _) = broadcast::channel(10_000);
		let (best_sender, _) = broadcast::channel(10_000);
		let (pool_sender, pool) = mpsc::channel(1);
		Self {
			finalized_sender,
			best_sender,
			pool_sender,
			pool: Arc::new(Mutex::new(pool)),
			executed: Default::default(),
			best_block: Default::default(),
			subscription_counter: Default::default(),
			force_stream_error: Default::default(),
		}
	}

	pub async fn submission(&self) -> u64 {
		self.pool.lock().await.next().await.unwrap()
	}

	pub async fn execute_tx(&self, nonce: u64, success: bool) {
		self.executed.lock().await.push(MockExtrinsic { nonce, success });
	}

	pub async fn make_block(&self) -> MockBlock {
		let mut best_block = self.best_block.lock().await;
		let extrinsics = std::mem::take(&mut *self.executed.lock().await);
		let id = BlockId {
			number: best_block.number + 1,
			hash: H256::random(),
		};
		let block = MockBlock { id, extrinsics };
		*best_block = id;
		self.best_sender.send(block.clone()).ok();
		self.finalized_sender.send(block.clone()).ok();
		block
	}

	pub async fn set_force_stream_error(&self, flag: bool) {
		*self.force_stream_error.lock().await = flag;
	}
}

#[async_trait::async_trait]
impl ITimechainClient for MockClient {
	type Submitter = MockSubmitter;
	type Block = MockBlock;
	type Update = ();
	async fn get_latest_block(&self) -> Result<BlockId> {
		Ok(*self.best_block.lock().await)
	}

	fn sign_payload<Call>(&self, _call: &Call, params: ExtrinsicParams) -> Vec<u8>
	where
		Call: TxPayload + Send + Sync,
	{
		let nonce = params.2 .0.unwrap_or_default();
		nonce.to_le_bytes().to_vec()
	}

	fn submittable_transaction(&self, tx: Vec<u8>) -> Self::Submitter {
		let nonce = u64::from_le_bytes(tx.try_into().unwrap());
		MockSubmitter {
			nonce,
			pool_sender: self.pool_sender.clone(),
		}
	}

	async fn finalized_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>
	{
		let rx = self.finalized_sender.subscribe();
		{
			let mut counter = self.subscription_counter.lock().await;
			*counter += 1;
		}
		let stream = BroadcastStream::new(rx)
			.map(|res| res.map_err(|e| anyhow::anyhow!(e)))
			.map(|block_result| block_result.map(|block| (block.clone(), block.extrinsics.clone())))
			.boxed();

		let stream = ErrorInjectingStream::new(stream, self.force_stream_error.clone()).boxed();
		Ok(stream)
	}

	async fn best_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>
	{
		let rx = self.best_sender.subscribe();
		{
			let mut counter = self.subscription_counter.lock().await;
			*counter += 1;
		}
		let stream = BroadcastStream::new(rx)
			.map(|res| res.map_err(|e| anyhow::anyhow!(e)))
			.map(|block_result| block_result.map(|block| (block.clone(), block.extrinsics.clone())))
			.boxed();
		Ok(stream)
	}

	async fn runtime_updates(&self) -> Result<BoxStream<'static, Result<Self::Update>>> {
		Ok(stream::empty().boxed())
	}

	fn apply_update(&self, _update: Self::Update) -> Result<()> {
		Ok(())
	}
}

pub struct MockSubmitter {
	nonce: u64,
	pool_sender: mpsc::Sender<u64>,
}

#[async_trait::async_trait]
impl ITransactionSubmitter for MockSubmitter {
	fn hash(&self) -> H256 {
		let mut hash = [0; 32];
		hash[..8].copy_from_slice(&self.nonce.to_le_bytes());
		hash.into()
	}

	async fn submit(&self) -> Result<H256> {
		self.pool_sender.clone().send(self.nonce).await?;
		Ok(self.hash())
	}
}

#[derive(Clone, Debug)]
pub struct MockBlock {
	pub id: BlockId,
	pub extrinsics: Vec<MockExtrinsic>,
}

#[async_trait::async_trait]
impl IBlock for MockBlock {
	type Extrinsic = MockExtrinsic;

	async fn extrinsics(&self) -> Result<Vec<Self::Extrinsic>> {
		Ok(self.extrinsics.clone())
	}

	fn number(&self) -> u64 {
		self.id.number
	}

	fn hash(&self) -> H256 {
		self.id.hash
	}
}

#[derive(Clone, Debug)]
pub struct MockExtrinsic {
	pub nonce: u64,
	pub success: bool,
}

#[async_trait::async_trait]
impl IExtrinsic for MockExtrinsic {
	type Events = ();

	async fn events(&self) -> Result<Self::Events> {
		Ok(())
	}

	fn hash(&self) -> H256 {
		let mut hash = [0; 32];
		hash[..8].copy_from_slice(&self.nonce.to_le_bytes());
		hash.into()
	}

	async fn is_success(&self) -> Result<()> {
		if self.success {
			Ok(())
		} else {
			anyhow::bail!("tx is failed")
		}
	}
}

#[derive(Default, Clone)]
pub struct MockDb {
	data: std::sync::Arc<std::sync::Mutex<HashMap<[u8; 32], TxData>>>,
}

impl ITransactionDbOps for MockDb {
	fn store_tx(&self, tx_data: &TxData) -> Result<()> {
		self.data.lock().unwrap().insert(tx_data.hash.0, tx_data.clone());
		Ok(())
	}

	fn remove_tx(&self, hash: H256) -> Result<()> {
		self.data.lock().unwrap().remove(&hash.0);
		Ok(())
	}

	fn load_pending_txs(&self, _nonce: u64) -> Result<VecDeque<TxData>> {
		let data = self.data.lock().unwrap();
		let mut txs: Vec<TxData> = data.values().cloned().collect();
		txs.sort_by_key(|tx| tx.nonce);
		Ok(txs.into())
	}
}

struct ErrorInjectingStream<S> {
	inner: S,
	flag: Arc<Mutex<bool>>,
	errored: bool,
}

impl<S> ErrorInjectingStream<S> {
	fn new(inner: S, flag: Arc<Mutex<bool>>) -> Self {
		Self { inner, flag, errored: false }
	}
}

impl<S, T> Stream for ErrorInjectingStream<S>
where
	S: Stream<Item = Result<T>> + Unpin,
{
	type Item = Result<T>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let flag_set = {
			if let Ok(lock) = self.flag.try_lock() {
				*lock
			} else {
				false
			}
		};

		if flag_set && !self.errored {
			self.errored = true;
			return Poll::Ready(Some(Err(anyhow::anyhow!("Forced stream error for testing"))));
		}

		let next = Pin::new(&mut self.inner).poll_next(cx);
		if self.errored {
			return Poll::Ready(None);
		}
		next
	}
}
