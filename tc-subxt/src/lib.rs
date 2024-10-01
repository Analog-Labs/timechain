#![allow(clippy::missing_transmute_annotations)]
use crate::worker::{SubxtWorker, Tx};
use anyhow::{Context, Result};
use futures::channel::{mpsc, oneshot};
use futures::stream::BoxStream;
use std::future::Future;
use std::str::FromStr;
use std::time::Duration;
use subxt::backend::rpc::reconnecting_rpc_client::{Client, ExponentialBackoff};
use subxt_signer::SecretUri;
use time_primitives::{AccountId, BlockHash, BlockNumber, PublicKey};

mod api;
mod metadata;
mod worker;

pub use crate::metadata::MetadataVariant;
pub use subxt_signer::sr25519::Keypair;

pub type OnlineClient = subxt::OnlineClient<subxt::PolkadotConfig>;
pub type LegacyRpcMethods = subxt::backend::legacy::LegacyRpcMethods<subxt::PolkadotConfig>;
pub type TxInBlock = subxt::tx::TxInBlock<subxt::PolkadotConfig, OnlineClient>;
pub type TxProgress = subxt::tx::TxProgress<subxt::PolkadotConfig, OnlineClient>;

#[derive(Clone)]
pub struct SubxtClient {
	client: OnlineClient,
	metadata: MetadataVariant,
	tx: mpsc::UnboundedSender<(Tx, oneshot::Sender<TxInBlock>)>,
	public_key: PublicKey,
	account_id: AccountId,
}

impl SubxtClient {
	pub async fn new(url: &str, metadata: MetadataVariant, keypair: Keypair) -> Result<Self> {
		let (rpc_client, client) = Self::get_client(url).await?;
		let worker = SubxtWorker::new(rpc_client, client.clone(), metadata, keypair).await?;
		let public_key = worker.public_key();
		let account_id = worker.account_id();
		tracing::info!("account id {}", account_id);
		let tx = worker.into_sender();
		Ok(Self {
			client,
			metadata,
			tx,
			public_key,
			account_id,
		})
	}

	pub async fn with_key(url: &str, metadata: MetadataVariant, mnemonic: &str) -> Result<Self> {
		let secret =
			SecretUri::from_str(mnemonic.trim()).context("failed to parse substrate keyfile")?;
		let keypair = Keypair::from_uri(&secret).context("substrate keyfile contains uri")?;
		Self::new(url, metadata, keypair).await
	}

	pub async fn get_client(url: &str) -> Result<(LegacyRpcMethods, OnlineClient)> {
		let rpc_client = Client::builder()
			.retry_policy(
				ExponentialBackoff::from_millis(100).max_delay(Duration::from_secs(10)).take(3),
			)
			.build(url.to_string())
			.await?;
		let rpc = LegacyRpcMethods::new(rpc_client.clone().into());
		let client = OnlineClient::from_rpc_client(rpc_client.clone())
			.await
			.map_err(|_| anyhow::anyhow!("Failed to create a new client"))?;
		Ok((rpc, client))
	}

	pub fn public_key(&self) -> &PublicKey {
		&self.public_key
	}

	pub fn account_id(&self) -> &AccountId {
		&self.account_id
	}

	pub async fn latest_block(&self) -> Result<u64> {
		Ok(self.client.blocks().at_latest().await?.number().into())
	}

	pub fn block_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		let client = self.client.clone();
		let f = move || client.blocks().subscribe_all();
		block_stream(f)
	}

	pub fn finality_notification_stream(&self) -> BoxStream<'static, (BlockHash, BlockNumber)> {
		let client = self.client.clone();
		let f = move || client.blocks().subscribe_finalized();
		block_stream(f)
	}

	pub async fn transfer(&self, account: AccountId, balance: u128) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Transfer { account, balance }, tx))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}

	pub async fn balance(&self, account: &AccountId) -> Result<u128> {
		metadata_scope!(self.metadata, {
			let storage_query =
				metadata::storage().system().account(&subxt::utils::Static(account.clone()));
			let result = self.client.storage().at_latest().await?.fetch(&storage_query).await?;
			Ok(if let Some(info) = result { info.data.free } else { 0 })
		})
	}
}

type Block = subxt::blocks::Block<subxt::PolkadotConfig, OnlineClient>;
type BlockStreamOutput = Result<subxt::backend::StreamOfResults<Block>, subxt::error::Error>;

fn block_stream<
	B: Future<Output = BlockStreamOutput> + Send + 'static,
	F: Fn() -> B + Send + 'static,
>(
	f: F,
) -> BoxStream<'static, (BlockHash, BlockNumber)> {
	let stream = async_stream::stream! {
		loop {
			let mut block_stream = match f().await {
				Ok(stream) => stream,
				Err(e) => {
					tracing::error!("Error subscribing to block stream {:?}", e);
					tokio::time::sleep(Duration::from_secs(1)).await;
					continue;
				},
			};
			while let Some(block_result) = block_stream.next().await {
				match block_result {
					Ok(block) => {
						let block_hash = block.hash();
						let block_number = block.header().number;
						yield (block_hash, block_number);
					},
					Err(subxt::error::Error::Rpc(_)) => break,
					Err(e) => {
						tracing::error!("Error receiving block: {:?}", e);
						tokio::time::sleep(Duration::from_secs(1)).await;
						continue;
					},
				}
			}
		}
	};
	Box::pin(stream)
}
