use crate::worker::TxData;
use crate::{metadata, ExtrinsicParams, OnlineClient, SubmittableExtrinsic};
use anyhow::{Context, Result};
use futures::{stream::BoxStream, StreamExt, TryStreamExt};
use metadata::runtime_types::sp_runtime::DispatchError;
use metadata::system::events::ExtrinsicFailed;
use metadata::technical_committee::events::MemberExecuted;
use std::collections::VecDeque;
use subxt::utils::H256;
use subxt::{
	client::{Update, UpgradeError},
	tx::Payload,
};
pub use subxt_signer::sr25519::Keypair;

#[derive(Clone, Copy, Debug, Default)]
pub struct BlockId {
	pub number: u64,
	pub hash: H256,
}

#[async_trait::async_trait]
pub trait ITimechainClient {
	type Submitter: ITransactionSubmitter + Send + Sync;
	type Block: IBlock + Send + Sync;
	type Update: Send + Sync;
	async fn get_latest_block(&self) -> Result<BlockId>;
	fn sign_payload<Call>(&self, call: &Call, params: ExtrinsicParams) -> Vec<u8>
	where
		Call: Payload + Send + Sync;
	fn submittable_transaction(&self, tx: Vec<u8>) -> Self::Submitter;
	async fn finalized_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>;
	async fn best_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>;
	async fn runtime_updates(&self) -> Result<BoxStream<'static, Result<Self::Update>>>;
	fn apply_update(&self, update: Self::Update) -> Result<()>;
}

#[async_trait::async_trait]
pub trait ITransactionSubmitter: Send + Sync {
	fn hash(&self) -> H256;
	async fn submit(&self) -> Result<H256>;
}

#[async_trait::async_trait]
pub trait IBlock: Send + Sync {
	type Extrinsic: IExtrinsic + Send + Sync;
	async fn extrinsics(&self) -> Result<Vec<Self::Extrinsic>>;
	fn number(&self) -> u64;
	fn hash(&self) -> H256;
}

#[async_trait::async_trait]
pub trait IExtrinsic: Send + Sync {
	type Events: Send + Sync;
	async fn events(&self) -> Result<Self::Events>;
	fn hash(&self) -> H256;
	async fn is_success(&self) -> Result<()>;
}

pub trait ITransactionDbOps: Send + Sync {
	fn store_tx(&self, tx_data: &TxData) -> Result<()>;
	fn remove_tx(&self, hash: H256) -> Result<()>;
	fn load_pending_txs(&self, nonce: u64) -> Result<VecDeque<TxData>>;
}

#[derive(Clone)]
pub struct TimechainOnlineClient {
	client: OnlineClient,
	keypair: Keypair,
}

impl TimechainOnlineClient {
	pub fn new(client: OnlineClient, keypair: Keypair) -> Self {
		Self { client, keypair }
	}
}
pub struct SignedTransaction {
	tx: SubmittableExtrinsic,
}

pub struct TimechainBlock {
	pub client: OnlineClient,
	pub block: crate::Block,
}

pub struct TimechainExtrinsic {
	pub client: OnlineClient,
	pub extrinsic: crate::ExtrinsicDetails,
}

pub struct TimechainEvents {
	pub events: crate::ExtrinsicEvents,
}

#[async_trait::async_trait]
impl ITimechainClient for TimechainOnlineClient {
	type Submitter = SignedTransaction;
	type Block = TimechainBlock;
	type Update = Update;

	async fn get_latest_block(&self) -> Result<BlockId> {
		let block = self.client.blocks().at_latest().await?;
		Ok(BlockId {
			number: block.number().into(),
			hash: block.hash(),
		})
	}

	fn sign_payload<Call>(&self, call: &Call, params: ExtrinsicParams) -> Vec<u8>
	where
		Call: Payload + Send + Sync,
	{
		self.client
			.tx()
			.create_partial_offline(call, params)
			.expect("Metadata is invalid")
			.sign(&self.keypair)
			.into_encoded()
	}

	fn submittable_transaction(&self, tx: Vec<u8>) -> Self::Submitter {
		tracing::debug!("Transaction prepared for submission: {}", hex::encode(&tx));
		let tx = SubmittableExtrinsic::from_bytes(self.client.clone(), tx);
		SignedTransaction { tx }
	}

	async fn finalized_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>
	{
		let finalized_stream = self.client.blocks().subscribe_finalized().await?;
		let client = self.client.clone();
		let stream_with_txs =
			finalized_stream
				.map(|res| res.map_err(anyhow::Error::new))
				.and_then(move |block| {
					let client = client.clone();
					async move {
						let block = TimechainBlock { client, block };
						let extrinsics = IBlock::extrinsics(&block).await?;
						Ok((block, extrinsics))
					}
				});
		Ok(stream_with_txs.boxed())
	}
	async fn best_block_stream(
		&self,
	) -> Result<BoxStream<'static, Result<(Self::Block, Vec<<Self::Block as IBlock>::Extrinsic>)>>>
	{
		let best_stream = self.client.blocks().subscribe_best().await?;
		let client = self.client.clone();
		let stream_with_txs =
			best_stream.map(|res| res.map_err(anyhow::Error::new)).and_then(move |block| {
				let client = client.clone();
				async move {
					let block = TimechainBlock { client, block };
					let extrinsics = IBlock::extrinsics(&block).await?;
					Ok((block, extrinsics))
				}
			});
		Ok(stream_with_txs.boxed())
	}
	async fn runtime_updates(&self) -> Result<BoxStream<'static, Result<Self::Update>>> {
		let updater = self.client.updater();
		let stream = updater.runtime_updates().await?;
		let stream = futures::stream::try_unfold(stream, |mut stream| async move {
			match stream.next().await {
				Some(Ok(update)) => Ok(Some((update, stream))),
				Some(Err(e)) => Err(e.into()),
				None => Ok(None),
			}
		});

		Ok(stream.boxed())
	}

	fn apply_update(&self, update: Self::Update) -> Result<()> {
		let updater = self.client.updater();
		let version = update.runtime_version().spec_version;
		if let Err(err) = updater.apply_update(update) {
			if !matches!(err, UpgradeError::SameVersion) {
				tracing::error!("Update to version {version} failed: {err:?}");
			}
		} else {
			tracing::info!("Updated to runtime version {version}");
		};
		Ok(())
	}
}

#[async_trait::async_trait]
impl ITransactionSubmitter for SignedTransaction {
	fn hash(&self) -> H256 {
		self.tx.hash()
	}
	async fn submit(&self) -> Result<H256> {
		self.tx.submit().await.map_err(|e| anyhow::anyhow!(e))
	}
}

#[async_trait::async_trait]
impl IBlock for TimechainBlock {
	type Extrinsic = TimechainExtrinsic;
	async fn extrinsics(&self) -> Result<Vec<Self::Extrinsic>> {
		let extrinsics = self.block.extrinsics().await?;
		Ok(extrinsics
			.iter()
			.map(|extrinsic| TimechainExtrinsic {
				client: self.client.clone(),
				extrinsic,
			})
			.collect())
	}
	fn number(&self) -> u64 {
		self.block.number().into()
	}
	fn hash(&self) -> H256 {
		self.block.hash()
	}
}

#[async_trait::async_trait]
impl IExtrinsic for TimechainExtrinsic {
	type Events = TimechainEvents;
	async fn events(&self) -> Result<Self::Events> {
		Ok(TimechainEvents {
			events: self.extrinsic.events().await?,
		})
	}
	fn hash(&self) -> H256 {
		self.extrinsic.hash()
	}
	async fn is_success(&self) -> Result<()> {
		let events = self.extrinsic.events().await?;
		for ev in events.iter() {
			let ev = ev?;

			let error = if let Some(ExtrinsicFailed { dispatch_error, .. }) =
				ev.as_event::<ExtrinsicFailed>()?
			{
				dispatch_error
			} else if let Some(MemberExecuted { result: Err(error), .. }) =
				ev.as_event::<MemberExecuted>()?
			{
				error
			} else {
				continue;
			};

			let DispatchError::Module(error) = error else {
				anyhow::bail!("tx failed with error: {:?}", error);
			};

			let metadata = self.client.metadata();
			let pallet = metadata.pallet_by_index_err(error.index)?;
			let error =
				pallet.error_variant_by_index(error.error[0]).context("unknown error variant")?;
			anyhow::bail!("tx failed with error: {}::{}", pallet.name(), error.name);
		}
		Ok(())
	}
}
