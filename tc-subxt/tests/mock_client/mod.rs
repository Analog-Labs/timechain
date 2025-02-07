use std::sync::Arc;

use anyhow::Result;
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use subxt::{tx::Payload as TxPayload, utils::H256};
use tc_subxt::timechain_client::{
	BlockDetail, IBlock, IExtrinsic, ITimechainClient, ITransactionSubmitter,
};
use tc_subxt::ExtrinsicParams;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
pub struct MockClient {
	pub subscription_counter: Arc<Mutex<u8>>,
	pub latest_block: Arc<Mutex<BlockDetail>>,
	submitted_hashes: Arc<Mutex<Vec<H256>>>,
	finalized_sender: broadcast::Sender<MockBlock>,
	best_sender: broadcast::Sender<MockBlock>,
}

impl Default for MockClient {
	fn default() -> Self {
		Self::new()
	}
}

impl MockClient {
	pub fn new() -> Self {
		let (finalized_sender, _) = broadcast::channel(1000);
		let (best_sender, _) = broadcast::channel(1000);
		Self {
			subscription_counter: Default::default(),
			finalized_sender,
			best_sender,
			latest_block: Arc::new(Mutex::new(BlockDetail {
				number: 0,
				hash: Default::default(),
			})),
			submitted_hashes: Arc::new(Mutex::new(Vec::new())),
		}
	}

	async fn push_finalized_block(&self, block: &MockBlock) {
		self.finalized_sender.send(block.clone()).ok();
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

	pub async fn inc_block(&self, tx_hash: Option<H256>) {
		let current_block = {
			let current_block = self.latest_block.lock().await;
			current_block.number
		};
		let mut block = MockBlock {
			number: current_block + 1,
			hash: H256::random(),
			extrinsics: vec![],
		};
		if let Some(hash) = tx_hash {
			block.extrinsics.push(MockExtrinsic::new(hash));
		}
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
		}
		tracing::info!("Done inserting blocks");
	}
}

pub struct MockTransaction {
	hash: H256,
	submitted_hashes: Arc<Mutex<Vec<H256>>>,
}

#[derive(Clone)]
pub struct MockBlock {
	pub number: u64,
	pub hash: H256,
	pub extrinsics: Vec<MockExtrinsic>,
}

#[derive(Clone)]
pub struct MockExtrinsic {
	pub hash: H256,
	pub is_success: bool,
}

impl MockExtrinsic {
	pub fn new(hash: H256) -> Self {
		Self { hash, is_success: true }
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
		let mut hashes = self.submitted_hashes.lock().await;
		hashes.push(self.hash);
		Ok(self.hash)
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
