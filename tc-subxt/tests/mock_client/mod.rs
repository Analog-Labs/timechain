use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::Result;
use futures::stream::{self, BoxStream};
use futures::{Stream, StreamExt};
use subxt::{tx::Payload as TxPayload, utils::H256};
use tc_subxt::timechain_client::{
	BlockDetail, IBlock, IExtrinsic, ITimechainClient, ITransactionDbOps, ITransactionSubmitter,
};
use tc_subxt::worker::TxData;
use tc_subxt::ExtrinsicParams;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
pub struct MockClient {
	pub subscription_counter: Arc<Mutex<u8>>,
	pub latest_block: Arc<Mutex<BlockDetail>>,
	submitted_hashes: Arc<Mutex<Vec<H256>>>,
	failing_hashes: Arc<Mutex<Vec<H256>>>,
	force_stream_error: Arc<Mutex<bool>>,
	pub finalized_sender: broadcast::Sender<MockBlock>,
	pub best_sender: broadcast::Sender<MockBlock>,
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
		Self {
			subscription_counter: Default::default(),
			finalized_sender,
			best_sender,
			latest_block: Arc::new(Mutex::new(BlockDetail {
				number: 0,
				hash: Default::default(),
			})),
			submitted_hashes: Arc::new(Mutex::new(Vec::new())),
			failing_hashes: Arc::new(Mutex::new(Vec::new())),
			force_stream_error: Arc::new(Mutex::new(false)),
		}
	}

	pub async fn push_finalized_block(&self, block: &MockBlock) {
		self.finalized_sender.send(block.clone()).ok();
	}

	pub async fn set_force_stream_error(&self, flag: bool) {
		*self.force_stream_error.lock().await = flag;
	}

	async fn push_best_block(&self, block: &MockBlock) {
		{
			let mut latest = self.latest_block.lock().await;
			latest.number = block.number;
			latest.hash = block.hash;
		}
		self.best_sender.send(block.clone()).ok();
	}

	pub async fn submitted_transactions(&self) -> Vec<H256> {
		self.submitted_hashes.lock().await.clone()
	}

	pub async fn inc_block_with_tx(&self, tx_hash: H256, success: bool) {
		let current_block = {
			let current_block = self.latest_block.lock().await;
			current_block.number
		};
		let mut block = MockBlock {
			number: current_block + 1,
			hash: H256::random(),
			extrinsics: vec![],
		};
		block.extrinsics.push(MockExtrinsic::new(tx_hash, success));
		self.push_best_block(&block).await;
		self.push_finalized_block(&block).await;
	}

	pub async fn inc_empty_blocks(&self, num: u8) {
		let mut current_block = {
			let current_block = self.latest_block.lock().await;
			current_block.number
		};
		tracing::info!("Inserting empty blocks: {}", num);
		for _ in 0..num {
			current_block += 1;
			let block = MockBlock {
				number: current_block,
				hash: H256::random(),
				extrinsics: vec![],
			};
			self.push_best_block(&block).await;
			self.push_finalized_block(&block).await;
			tokio::time::sleep(Duration::from_millis(100)).await;
		}
		tracing::info!("Done inserting blocks");
	}

	pub async fn failing_transactions(&self, hash: &H256) {
		self.failing_hashes.lock().await.push(*hash);
	}
}

pub struct MockTransaction {
	hash: H256,
	submitted_hashes: Arc<Mutex<Vec<H256>>>,
	failing_hashes: Arc<Mutex<Vec<H256>>>,
}

#[derive(Clone, Debug)]
pub struct MockBlock {
	pub number: u64,
	pub hash: H256,
	pub extrinsics: Vec<MockExtrinsic>,
}

#[derive(Clone, Debug)]
pub struct MockExtrinsic {
	pub hash: H256,
	pub is_success: bool,
}

impl MockExtrinsic {
	pub fn new(hash: H256, is_success: bool) -> Self {
		Self { hash, is_success }
	}
}

#[async_trait::async_trait]
impl ITimechainClient for MockClient {
	type Submitter = MockTransaction;
	type Block = MockBlock;
	type Update = ();
	async fn get_latest_block(&self) -> Result<BlockDetail> {
		Ok(self.latest_block.lock().await.clone())
	}

	fn sign_payload<Call>(&self, _call: &Call, params: ExtrinsicParams) -> Vec<u8>
	where
		Call: TxPayload + Send + Sync,
	{
		let nonce = params.2 .0.unwrap_or_default();
		let mut bytes = [0u8; 32];
		bytes[24..].copy_from_slice(&nonce.to_le_bytes());
		bytes.to_vec()
	}

	fn submittable_transaction(&self, tx: Vec<u8>) -> Self::Submitter {
		let mut bytes = [0u8; 32];
		bytes.copy_from_slice(&tx[..32]);
		MockTransaction {
			hash: H256::from(bytes),
			submitted_hashes: self.submitted_hashes.clone(),
			failing_hashes: self.failing_hashes.clone(),
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
		let stream: BoxStream<'static, Result<Self::Update>> = stream::empty().boxed();
		Ok(stream)
	}
	fn apply_update(&self, _update: Self::Update) -> Result<()> {
		Ok(())
	}
}

#[async_trait::async_trait]
impl ITransactionSubmitter for MockTransaction {
	fn hash(&self) -> H256 {
		self.hash
	}
	async fn submit(&self) -> Result<H256> {
		let failing_hashes = self.failing_hashes.lock().await;
		let mut hashes = self.submitted_hashes.lock().await;
		hashes.push(self.hash);
		if failing_hashes.contains(&self.hash) {
			anyhow::bail!("Tx failed")
		} else {
			Ok(self.hash)
		}
	}
}

#[async_trait::async_trait]
impl IBlock for MockBlock {
	type Extrinsic = MockExtrinsic;
	async fn extrinsics(&self) -> Result<Vec<Self::Extrinsic>> {
		Ok(self.extrinsics.clone())
	}
	fn number(&self) -> u64 {
		self.number
	}
	fn hash(&self) -> H256 {
		self.hash
	}
}

#[async_trait::async_trait]
impl IExtrinsic for MockExtrinsic {
	type Events = ();
	async fn events(&self) -> Result<Self::Events> {
		Ok(())
	}

	fn hash(&self) -> H256 {
		self.hash
	}
	async fn is_success(&self) -> Result<()> {
		if self.is_success {
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

pub fn compute_tx_hash(index: u64) -> H256 {
	let mut bytes = [0u8; 32];
	bytes[24..].copy_from_slice(&index.to_le_bytes());
	bytes.into()
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
