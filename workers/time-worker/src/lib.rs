#![allow(clippy::type_complexity)]
mod communication;
mod worker;

#[cfg(test)]
mod tests;

use crate::communication::{time_protocol_name::gossip_protocol_name, validator::GossipValidator};
use futures::channel::mpsc;
use log::*;
use sc_client_api::{Backend, BlockchainEvents};
use sc_network_gossip::{GossipEngine, Network as GossipNetwork, Syncing as GossipSyncing};
use sp_api::ProvideRuntimeApi;
use sp_consensus::SyncOracle;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;

pub use crate::communication::time_protocol_name;
pub use crate::worker::{TssId, TssRequest};

/// Constant to indicate target for logging
pub const TW_LOG: &str = "time-worker";

/// Set of properties we need to run our gadget
pub struct TimeWorkerParams<B: Block, R, BE, N, S>
where
	B: Block + 'static,
	BE: Backend<B> + 'static,
	R: BlockchainEvents<B> + ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	S: GossipSyncing<B> + SyncOracle + 'static,
{
	pub _block: PhantomData<B>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_network: N,
	pub kv: KeystorePtr,
	pub sign_data_receiver: mpsc::Receiver<TssRequest>,
	pub sync_service: Arc<S>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_timeworker_gadget<B, R, BE, N, S>(
	timeworker_params: TimeWorkerParams<B, R, BE, N, S>,
) where
	B: Block + 'static,
	BE: Backend<B> + 'static,
	R: BlockchainEvents<B> + ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	S: GossipSyncing<B> + SyncOracle + 'static,
{
	debug!(target: TW_LOG, "Starting TimeWorker gadget");
	let TimeWorkerParams {
		_block,
		backend,
		runtime,
		gossip_network,
		kv,
		sign_data_receiver,
		sync_service,
	} = timeworker_params;
	let gossip_engine = GossipEngine::new(
		gossip_network,
		sync_service,
		gossip_protocol_name(),
		Arc::new(GossipValidator::new()),
		None,
	);
	let worker_params = worker::WorkerParams {
		_block,
		backend,
		runtime,
		gossip_engine,
		kv,
		sign_data_receiver,
	};
	let mut worker = worker::TimeWorker::new(worker_params);
	worker.run().await
}
