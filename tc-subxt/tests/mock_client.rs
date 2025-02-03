use anyhow::Result;
use futures::stream::BoxStream;
use subxt::{tx::Payload as TxPayload, utils::H256};
use tc_subxt::timechain_client::{
	BlockDetail, IBlock, IExtrinsic, ITimechainClient, ITransactionSubmitter,
};
use tc_subxt::ExtrinsicParams;

pub struct MockClient;
pub struct MockTransaction {
	hash: H256,
}
pub struct MockBlock;
pub struct MockExtrinsic;
pub struct MockEvent;

#[async_trait::async_trait]
impl ITransactionSubmitter for MockTransaction {
	fn hash(&self) -> H256 {
		self.hash
	}
	async fn submit(&self) -> Result<H256> {
		Ok(self.hash)
	}
}

#[async_trait::async_trait]
impl ITimechainClient for MockClient {
	type Submitter = MockTransaction;
	type Block = MockBlock;
	async fn get_latest_block(&self) -> Result<BlockDetail> {
		todo!()
	}
	fn sign_payload<Call>(&self, call: &Call, params: ExtrinsicParams) -> Vec<u8>
	where
		Call: TxPayload + Send + Sync,
	{
		todo!()
	}
	fn submittable_transaction(&self, tx: Vec<u8>) -> Self::Submitter {
		MockTransaction { hash: H256::random() }
	}
	async fn finalized_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>
	{
		todo!()
	}
	async fn best_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>
	{
		todo!()
	}
	async fn runtime_updates(&self) -> Result<BoxStream<'static, Result<Self::Update>>> {
		todo!()
	}
	async fn apply_update(&self, update: Self::Update) -> Result<()> {
		todo!()
	}
}

#[async_trait::async_trait]
impl IBlock for MockBlock {
	type Extrinsic = MockExtrinsic;
	async fn extrinsics(&self) -> Result<Vec<Self::Extrinsic>> {
		todo!()
	}
	fn number(&self) -> u64 {
		todo!()
	}
	fn hash(&self) -> H256 {
		todo!()
	}
}

#[async_trait::async_trait]
impl IExtrinsic for MockExtrinsic {
	type Events = MockEvent;
	async fn events(&self) -> Result<Self::Events> {
		todo!()
	}

	fn hash(&self) -> H256 {
		todo!()
	}
	async fn is_success(&self) -> Result<()> {
		todo!()
	}
}
