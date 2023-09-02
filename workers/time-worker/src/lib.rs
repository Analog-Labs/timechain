#![allow(clippy::type_complexity)]
pub mod tx_submitter;
mod worker;

#[cfg(test)]
mod tests;

use futures::channel::mpsc;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::config::{IncomingRequest, RequestResponseConfig};
use sc_network::NetworkRequest;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, time::Duration};
use time_primitives::{
	MembersApi, PeerId, PublicKey, ShardsApi, SubmitMembers, SubmitShards, TaskExecutor, TssRequest,
};

pub use worker::{TimeWorker, TimeWorkerParams};

/// Constant to indicate target for logging
pub const TW_LOG: &str = "time-worker";

/// time protocol name suffix.
pub const PROTOCOL_NAME: &str = "/time/1";

pub fn protocol_config(tx: async_channel::Sender<IncomingRequest>) -> RequestResponseConfig {
	RequestResponseConfig {
		name: PROTOCOL_NAME.into(),
		fallback_names: vec![],
		max_request_size: 1024 * 1024,
		max_response_size: 0,
		request_timeout: Duration::from_secs(3),
		inbound_queue: Some(tx),
	}
}
