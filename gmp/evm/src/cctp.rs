use alloy::sol_types::SolValue;
use anyhow::Result;
use futures::future::BoxFuture;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter};
use reqwest::Client;
use serde::Deserialize;
use sha3::Digest;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use time_primitives::{Address32, GmpMessage};

type CctpRetryCount = u8;
const MAX_CCTP_RETRY: CctpRetryCount = 10;
type CircleRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

#[derive(Deserialize, Debug)]
struct AttestationResponse {
	status: String,
	attestation: Option<String>,
}

alloy::sol! {
	#[derive(Debug, Default, PartialEq, Eq)]
	struct CCTP {
		/// The attestation (obs: will be provided by the chronicle).
		bytes attestation;
		/// The message bytes emitted by the MessageSent event (must be provided).
		bytes message;
		/// Extra data field used by cctp implementers for custom usage
		bytes extraData;
	}
}

impl CCTP {
	pub fn get_version(&self) -> anyhow::Result<u32> {
		if self.message.len() < 4 {
			return Err(anyhow::anyhow!("Message is too short to contain a version field"));
		}
		let version_bytes: [u8; 4] = self.message[0..4]
			.try_into()
			.map_err(|_| anyhow::anyhow!("Failed to extract version bytes"))?;
		let version = u32::from_be_bytes(version_bytes);
		Ok(version)
	}
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
		let burn_hash: [u8; 32] = sha3::Keccak256::digest(burn_message).into();
		tracing::info!("cctp_burn_msg: {}", hex::encode(burn_message));
		tracing::info!("cctp_burn_hash: {}", hex::encode(burn_hash));
		let url = url.trim_end_matches('/');
		let url = format!("{}/0x{}", url, hex::encode(burn_hash));
		Some(Self { msg, url, retry_count: 0, cctp })
	}

	async fn fetch_attestation(&self) -> Result<Vec<u8>> {
		let url = self.url.clone();
		let handle = tokio::task::spawn(async move {
			let client = Client::new();
			let response = client.get(&url).send().await?.error_for_status()?;
			let attestation_response: AttestationResponse = response.json().await?;
			if attestation_response.status != "complete" {
				anyhow::bail!("attestation pending");
			}
			let signature = attestation_response.attestation.unwrap_or_default();
			let signature = signature.strip_prefix("0x").unwrap_or(&signature);
			let attestation = hex::decode(signature)?;
			Ok(attestation)
		});
		handle.await?
	}

	fn attest(mut self, attestation: Vec<u8>) -> GmpMessage {
		self.cctp.attestation = attestation.into();
		self.msg.bytes = self.cctp.abi_encode();
		self.msg
	}
}

type AttestationFuture = BoxFuture<'static, (CctpMessage, Result<Vec<u8>>)>;

pub struct CctpHandler {
	queue: Mutex<FuturesUnordered<AttestationFuture>>,
	rate_limiter: Arc<CircleRateLimiter>,
}

impl CctpHandler {
	pub fn new() -> Self {
		// API Service Rate Limit
		// The CCTP API service rate limit is 35 requests per second. If you exceed 35 requests per second,
		// the service blocks all API requests for the next 5 minutes and returns an HTTP 429 response.
		// Going for 30 rps just for additional safety
		let quota = Quota::per_second(NonZeroU32::new(30).unwrap());
		Self {
			queue: Default::default(),
			rate_limiter: Arc::new(RateLimiter::direct(quota)),
		}
	}
	pub fn needs_attestation(
		&self,
		msg: &GmpMessage,
		info: Option<&(Vec<Address32>, String)>,
	) -> bool {
		if let Some((ref cctp_contracts, ref url)) = info {
			if cctp_contracts.contains(&msg.src) {
				if let Some(msg) = CctpMessage::new(msg.clone(), url) {
					tracing::info!("read cctp message {}", hex::encode(msg.msg.message_id()));
					let limiter = self.rate_limiter.clone();
					self.queue.lock().unwrap().push(
						async move {
							limiter.until_ready().await;
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
				let limiter = self.rate_limiter.clone();
				self.queue.lock().unwrap().push(
					async move {
						limiter.until_ready().await;
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
