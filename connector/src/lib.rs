use alloy_primitives::B256;
use alloy_sol_types::SolEvent;
use anyhow::Result;
use futures::stream::Stream;
use futures::FutureExt;
use rosetta_client::client::GenericClientStream;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{query::GetLogs, FilterBlockOption};
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use std::future::Future;
use std::num::NonZeroU64;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use time_primitives::{
	Gateway, GatewayMessage, GmpMessage, IConnector, NetworkId, TssPublicKey, TssSignature, TxHash,
};
use tokio::sync::Mutex;

pub mod admin;
pub(crate) mod sol;
pub(crate) mod ufloat;

#[derive(Clone)]
pub struct Connector {
	wallet: Arc<Wallet>,
	guard: Arc<Mutex<()>>,
	network_id: NetworkId,
}

impl Connector {
	pub(crate) fn wallet(&self) -> &Wallet {
		&self.wallet
	}
}

#[async_trait::async_trait]
impl IConnector for Connector {
	async fn new(
		network_id: NetworkId,
		blockchain: &str,
		network: &str,
		url: &str,
		keyfile: &Path,
	) -> Result<Self> {
		let wallet =
			Arc::new(Wallet::new(blockchain.parse()?, network, url, Some(keyfile), None).await?);
		Ok(Self {
			wallet,
			guard: Arc::new(Mutex::new(())),
			network_id,
		})
	}

	fn network_id(&self) -> NetworkId {
		self.network_id
	}

	fn account(&self) -> &str {
		&self.wallet.account().address
	}

	/// Checks if wallet have enough balance
	async fn balance(&self) -> Result<u128> {
		self.wallet.balance().await
	}

	async fn read_messages(
		&self,
		gateway: Gateway,
		from_block: Option<NonZeroU64>,
		to_block: u64,
	) -> Result<Vec<GmpMessage>> {
		self.wallet
			.query(GetLogs {
				contracts: vec![sol::addr(gateway).0 .0.into()],
				topics: vec![sol::IGateway::GmpCreated::SIGNATURE_HASH.0.into()],
				block: FilterBlockOption::Range {
					from_block: from_block.map(|x| x.get().into()),
					to_block: Some(to_block.into()),
				},
			})
			.await?
			.into_iter()
			.map(|log| {
				// if any GmpCreated message is received collect them and returns
				let topics =
					log.topics.into_iter().map(|topic| B256::from(topic.0)).collect::<Vec<_>>();
				let log = alloy_primitives::Log::new(
					sol::addr(gateway).into(),
					topics,
					log.data.0.to_vec().into(),
				)
				.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
				let log = sol::IGateway::GmpCreated::decode_log(&log, true)?;
				let msg = sol::event_to_gmp_message(log.data, self.network_id);
				Ok::<_, anyhow::Error>(msg)
			})
			.collect::<Result<Vec<_>, _>>()
	}

	async fn submit_messages(
		&self,
		gateway: Gateway,
		_msg: GatewayMessage,
		_signer: TssPublicKey,
		_sig: TssSignature,
	) -> Result<Vec<TxHash>, String> {
		// TODO: construct input
		let input = vec![];
		let gas_limit = 0;
		let _guard = self.guard.lock().await;
		let hash = self
			.wallet
			.eth_send_call(sol::addr(gateway).into(), input, 0, None, Some(gas_limit))
			.await
			.map(|s| s.tx_hash().0)
			.map_err(|err| err.to_string())?;
		Ok(vec![hash])
	}

	async fn verify_submission(&self, _msg: GatewayMessage, txs: Vec<TxHash>) -> Result<Vec<u8>> {
		let mut json = vec![];
		for tx in txs {
			let Some(receipt) = self.wallet.eth_transaction_receipt(tx).await? else {
				anyhow::bail!("transaction receipt from tx {} not found", hex::encode(tx));
			};
			if receipt.status_code != Some(1) {
				anyhow::bail!("transaction reverted");
			}
			json.push(serde_json::json!({
				"transactionHash": format!("{:?}", receipt.transaction_hash),
				"transactionIndex": receipt.transaction_index,
				"blockHash": receipt.block_hash.0.map(|block_hash| format!("{block_hash:?}")),
				"blockNumber": receipt.block_number,
				"from": format!("{:?}", receipt.from),
				"to": receipt.to.map(|to| format!("{to:?}")),
				"gasUsed": receipt.gas_used.map(|gas_used| format!("{gas_used:?}")),
				"contractAddress": receipt.contract_address.map(|contract| format!("{contract:?}")),
				"status": receipt.status_code,
				"type": receipt.transaction_type,
			}));
		}
		Ok(serde_json::json!(json).to_string().into_bytes())
	}

	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		Box::pin(BlockStream::new(&self.wallet))
	}
}

#[allow(clippy::type_complexity)]
struct BlockStream<'a> {
	wallet: &'a Arc<Wallet>,
	opening:
		Option<Pin<Box<dyn Future<Output = Result<Option<GenericClientStream<'a>>>> + Send + 'a>>>,
	listener: Option<GenericClientStream<'a>>,
}

impl<'a> BlockStream<'a> {
	pub fn new(wallet: &'a Arc<Wallet>) -> Self {
		Self {
			wallet,
			opening: None,
			listener: None,
		}
	}
}

impl<'a> Stream for BlockStream<'a> {
	type Item = u64;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		loop {
			if let Some(listener) = self.listener.as_mut() {
				match Pin::new(listener).poll_next(cx) {
					Poll::Ready(Some(event)) => match event {
						ClientEvent::NewFinalized(BlockOrIdentifier::Identifier(identifier)) => {
							return Poll::Ready(Some(identifier.index));
						},
						ClientEvent::NewFinalized(BlockOrIdentifier::Block(block)) => {
							return Poll::Ready(Some(block.block_identifier.index));
						},
						ClientEvent::NewHead(_) => continue,
						ClientEvent::Event(_) => continue,
						ClientEvent::Close(reason) => {
							tracing::warn!("block stream closed {}", reason);
							self.listener.take();
						},
					},
					Poll::Ready(None) => {
						self.listener.take();
					},
					Poll::Pending => return Poll::Pending,
				}
			}
			if let Some(opening) = self.opening.as_mut() {
				match Pin::new(opening).poll(cx) {
					Poll::Ready(Ok(Some(stream))) => {
						self.opening.take();
						self.listener = Some(stream);
					},
					Poll::Ready(Ok(None)) => {
						self.opening.take();
						tracing::debug!("error opening listener");
					},
					Poll::Ready(Err(err)) => {
						self.opening.take();
						tracing::debug!("error opening listener {}", err);
					},
					Poll::Pending => return Poll::Pending,
				}
			}
			let wallet = self.wallet;
			self.opening = Some(wallet.listen().boxed());
		}
	}
}
