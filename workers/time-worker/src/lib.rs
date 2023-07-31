#![allow(clippy::type_complexity)]
mod communication;
mod traits;
mod worker;

#[cfg(test)]
mod tests;

use log::*;
use sc_client_api::Backend;
use sc_network_gossip::{Network as GossipNetwork, Syncing as GossipSyncing};
use sp_api::ProvideRuntimeApi;
use sp_consensus::SyncOracle;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;
use traits::Client;

pub use crate::communication::time_protocol_name;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "time-worker";

/// Set of properties we need to run our gadget
pub struct TimeWorkerParams<B: Block, A, BN, C, R, BE, N, S>
where
	B: Block + 'static,
	A: sp_runtime::codec::Codec + 'static,
	BN: sp_runtime::codec::Codec + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B, A, BN>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	S: GossipSyncing<B> + SyncOracle + 'static,
{
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_network: N,
	pub kv: KeystorePtr,
	pub _block: PhantomData<B>,
	pub accountid: PhantomData<A>,
	pub _block_number: PhantomData<BN>,
	pub sync_service: Arc<S>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_timeworker_gadget<B, A, BN, C, R, BE, N, S>(
	params: TimeWorkerParams<B, A, BN, C, R, BE, N, S>,
) where
	B: Block + 'static,
	A: sp_runtime::codec::Codec + 'static,
	BN: sp_runtime::codec::Codec + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B, A, BN>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	S: GossipSyncing<B> + SyncOracle + 'static,
{
	debug!(target: TW_LOG, "Starting TimeWorker gadget");
	let mut worker = worker::TimeWorker::new(params).await;
	worker.run().await
}
