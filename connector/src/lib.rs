//use alloy_primitives::B256;
//use alloy_sol_types::SolEvent;
use anyhow::Result;
use futures::stream::Stream;
use futures::FutureExt;
use rosetta_client::client::GenericClientStream;
use rosetta_client::Wallet;
use rosetta_config_ethereum::{query::GetLogs, CallResult, FilterBlockOption};
use rosetta_core::{BlockOrIdentifier, ClientEvent};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use time_primitives::{GatewayMessage, GmpMessage};
use tokio::sync::Mutex;
use tokio::time::sleep;

pub struct Connector {
	wallet: Arc<Wallet>,
	guard: Arc<Mutex<()>>,
	min_balance: u128,
}

impl Connector {
	pub async fn new(
		blockchain: &str,
		network: &str,
		url: &str,
		keyfile: &Path,
		min_balance: u128,
	) -> Result<Self> {
		let wallet =
			Arc::new(Wallet::new(blockchain.parse()?, network, url, Some(keyfile), None).await?);

		let connector = Self {
			wallet,
			guard: Arc::new(Mutex::new(())),
			min_balance,
		};

		while !connector.is_balance_available().await? {
			sleep(Duration::from_secs(10)).await;
			tracing::warn!("Chronicle balance is too low, retrying...");
		}

		Ok(connector)
	}

	pub fn target_address(&self) -> &str {
		&self.wallet.account().address
	}

	/// Checks if wallet have enough balance
	async fn is_balance_available(&self) -> Result<bool> {
		let balance = self.wallet.balance().await?;
		Ok(balance > self.min_balance)
	}

	/*
	/// look for gmp events and filter GmpCreated events from them
	async fn get_gmp_events_at(
		&self,
		gateway_contract: [u8; 20],
		from_block: Option<NonZeroU64>,
		to_block: u64,
	) -> Result<Vec<IGateway::GmpCreated>> {
		let logs = self
			.wallet
			.query(GetLogs {
				contracts: vec![gateway_contract.into()],
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
				let log = alloy_primitives::Log::new(
					gateway_contract.into(),
					topics,
					log.data.0.to_vec().into(),
				)
				.ok_or_else(|| anyhow::format_err!("failed to decode log"))?;
				let log = IGateway::GmpCreated::decode_log(&log, true)?;
				Ok::<IGateway::GmpCreated, anyhow::Error>(log.data)
			})
			.collect::<Result<Vec<_>, _>>()?;
		Ok(logs)
	}*/

	/*
	///
	/// executes the task function
	///
	/// # Arguments
	/// `function`: function of task
	/// `target_block_number`: block number of target chain (usable for read tasks)
	async fn execute_function(
		&self,
		function: &Function,
		target_block_number: u64,
	) -> Result<Payload> {
		Ok(match function {
			// execute the read function of task
			Function::EvmViewCall { address, input } => {
				let data = self
					.wallet
					.eth_view_call(*address, input.clone(), target_block_number.into())
					.await?;
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
				Payload::Hashed(VerifyingKey::message_hash(json.to_string().as_bytes()))
			},
			// reads the receipt for a transaction on target chain
			Function::EvmTxReceipt { tx } => {
				let Some(receipt) = self.wallet.eth_transaction_receipt(*tx).await? else {
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
				})
				.to_string();
				Payload::Hashed(VerifyingKey::message_hash(json.to_string().as_bytes()))
			},
			// executs the read message function. it looks for event emitted from gateway contracts
			Function::ReadMessages { batch_size } => {
				let network_id = self.network_id;
				let Some(gateway_contract) = self.substrate.get_gateway(network_id).await? else {
					anyhow::bail!("no gateway registered: skipped reading messages");
				};
				// gets gmp created events form contract and then convert it to a `Msg`
				let logs: Vec<_> = self
					.get_gmp_events_at(
						gateway_contract,
						NonZeroU64::new(target_block_number - batch_size.get() + 1),
						target_block_number,
					)
					.await
					.context("get_gmp_events_at")?
					.into_iter()
					.map(|event| Msg::from_event(event, network_id))
					.collect();
				tracing::info!("read target block: {:?}", target_block_number);
				if !logs.is_empty() {
					tracing::info!("read {} messages", logs.len());
				}
				Payload::Gmp(logs)
			},
			_ => anyhow::bail!("not a read function {function:?}"),
		})
	}*/

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
