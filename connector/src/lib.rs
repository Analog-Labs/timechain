use crate::gateway::IGateway;
use alloy_primitives::B256;
use alloy_sol_types::SolEvent;
use anyhow::Result;
use futures::stream::Stream;
use futures::FutureExt;
use rosetta_client::client::GenericClientStream;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{query::GetLogs, CallResult, FilterBlockOption};
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use std::future::Future;
use std::num::NonZeroU64;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use time_primitives::{GatewayMessage, GmpMessage, GmpParams, NetworkId, TssSignature};
use tokio::sync::Mutex;

mod gateway;

#[derive(Clone)]
pub struct Connector {
	wallet: Arc<Wallet>,
	guard: Arc<Mutex<()>>,
	network_id: NetworkId,
}

impl Connector {
	pub async fn new(
		blockchain: &str,
		network: &str,
		url: &str,
		keyfile: &Path,
		network_id: NetworkId,
	) -> Result<Self> {
		let wallet =
			Arc::new(Wallet::new(blockchain.parse()?, network, url, Some(keyfile), None).await?);
		Ok(Self {
			wallet,
			guard: Arc::new(Mutex::new(())),
			network_id,
		})
	}

	pub fn network_id(&self) -> NetworkId {
		self.network_id
	}

	pub fn account(&self) -> &str {
		&self.wallet.account().address
	}

	/// Checks if wallet have enough balance
	pub async fn balance(&self) -> Result<u128> {
		self.wallet.balance().await
	}

	pub async fn read_messages(
		&self,
		gateway: [u8; 20],
		from_block: Option<NonZeroU64>,
		to_block: u64,
	) -> Result<Vec<GmpMessage>> {
		self.wallet
			.query(GetLogs {
				contracts: vec![gateway.into()],
				topics: vec![IGateway::GmpCreated::SIGNATURE_HASH.0.into()],
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
				let log =
					alloy_primitives::Log::new(gateway.into(), topics, log.data.0.to_vec().into())
						.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
				let log = IGateway::GmpCreated::decode_log(&log, true)?;
				let msg = gateway::event_to_gmp_message(log.data, self.network_id);
				Ok::<_, anyhow::Error>(msg)
			})
			.collect::<Result<Vec<_>, _>>()
	}

	pub async fn submit_gateway_message(
		&self,
		params: GmpParams,
		msg: GatewayMessage,
		sig: TssSignature,
	) -> Result<[u8; 32], String> {
		// TODO: construct input
		let input = vec![];
		let gas_limit = 0;
		let _guard = self.guard.lock().await;
		self.wallet
			.eth_send_call(params.gateway, input, 0, None, Some(gas_limit))
			.await
			.map(|s| s.tx_hash().0)
			.map_err(|err| err.to_string())
	}

	pub async fn verify_gateway_message_receipt(&self, tx: [u8; 32]) -> Result<Vec<u8>> {
		let Some(receipt) = self.wallet.eth_transaction_receipt(tx).await? else {
			anyhow::bail!("transaction receipt from tx {} not found", hex::encode(tx));
		};
		if receipt.status_code != Some(1) {
			anyhow::bail!("transaction reverted");
		}
		let json = serde_json::json!({
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
		});
		Ok(json.to_string().into_bytes())
	}

	pub async fn evm_view_call(
		&self,
		address: [u8; 20],
		input: Vec<u8>,
		target_block: u64,
	) -> Result<Vec<u8>> {
		let data = self.wallet.eth_view_call(address, input, target_block.into()).await?;
		let json = match data {
			// Call executed successfully
			CallResult::Success(data) => serde_json::json!({
				"success": hex::encode(data),
			}),
			// Call reverted
			CallResult::Revert(data) => serde_json::json!({
				"revert": hex::encode(data),
			}),
			// Call invalid or EVM error
			CallResult::Error => serde_json::json!({
				"error": null,
			}),
		};
		Ok(json.to_string().into_bytes())
	}

	pub fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
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
