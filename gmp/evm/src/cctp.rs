use crate::sol::CCTP;
use alloy::sol_types::SolValue;
use anyhow::Result;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use sha3::Digest;
use std::sync::Mutex;
use time_primitives::{Address32, GmpMessage};

type CctpRetryCount = u8;
const MAX_CCTP_RETRY: CctpRetryCount = 3;

#[derive(Deserialize, Debug)]
struct AttestationResponse {
	status: String,
	attestation: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CctpMessage {
	url: String,
	retry_count: CctpRetryCount,
	msg: GmpMessage,
	cctp: CCTP,
}

impl CctpMessage {
	fn new(msg: GmpMessage, url: &str) -> Option<Self> {
		let cctp = CCTP::abi_decode(&msg.bytes).ok()?;
		let version = cctp.get_version().ok()?;
		if version != 0 {
			tracing::error!("unsupported cctp version {version}");
			return None;
		}
		let burn_message = &cctp.message;
		let burn_hash: [u8; 32] = sha3::Keccak256::digest(&burn_message).into();
		let url = url.trim_end_matches('/');
		let url = format!("{}/0x{}", url, hex::encode(burn_hash));
		Some(Self { msg, url, retry_count: 0, cctp })
	}

	async fn fetch_attestation(&self) -> Result<Vec<u8>> {
		let client = Client::new();
		let response = client.get(&self.url).send().await?.error_for_status()?;
		let attestation_response: AttestationResponse = response.json().await?;
		if attestation_response.status != "complete" {
			anyhow::bail!("attestation pending");
		}
		let signature = attestation_response.attestation.unwrap_or_default();
		let signature = signature.strip_prefix("0x").unwrap_or(&signature);
		let attestation = hex::decode(signature)?;
		Ok(attestation)
	}

	fn attest(mut self, attestation: Vec<u8>) -> GmpMessage {
		self.cctp.attestation = attestation.into();
		self.msg.bytes = self.cctp.abi_encode();
		self.msg
	}
}

#[derive(Default)]
pub struct CctpHandler {
	queue: Mutex<FuturesUnordered<BoxFuture<'static, (CctpMessage, Result<Vec<u8>>)>>>,
}

impl CctpHandler {
	pub fn needs_attestation(
		&self,
		msg: &GmpMessage,
		info: Option<&(Vec<Address32>, String)>,
	) -> bool {
		if let Some((ref cctp_contracts, ref url)) = info {
			if cctp_contracts.contains(&msg.src) {
				if let Some(msg) = CctpMessage::new(msg.clone(), url) {
					tracing::info!("read cctp message {}", hex::encode(msg.msg.message_id()));
					self.queue.lock().unwrap().push(
						async move {
							let result = msg.fetch_attestation().await;
							(msg, result)
						}
						.boxed(),
					);
					return true;
				}
			}
		}
		false
	}

	pub async fn pop_attested(&self) -> Option<GmpMessage> {
		let (mut msg, result) = self.queue.lock().unwrap().next().now_or_never()??;
		match result {
			Ok(attestation) => return Some(msg.attest(attestation)),
			Err(error) if msg.retry_count >= MAX_CCTP_RETRY => {
				tracing::error!(
					"failed to fetch attestation #{} due to {error:?}, sending msg unattested",
					msg.retry_count
				);
				return Some(msg.attest(vec![]));
			},
			Err(error) => {
				msg.retry_count += 1;
				tracing::info!(
					"failed to fetch attestation #{} due to {error:?}, retrying",
					msg.retry_count
				);
				self.queue.lock().unwrap().push(
					async move {
						let result = msg.fetch_attestation().await;
						(msg, result)
					}
					.boxed(),
				);
			},
		};
		None
	}
}
