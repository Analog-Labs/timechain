#![allow(clippy::type_complexity)]
mod communication;
mod traits;
mod worker;

#[cfg(test)]
mod tests;

use crate::communication::{time_protocol_name::gossip_protocol_name, validator::GossipValidator};
use futures::channel::mpsc;
use log::*;
use sc_client_api::Backend;
use sc_network_gossip::{GossipEngine, Network as GossipNetwork, Syncing as GossipSyncing};
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
	pub sign_data_receiver: mpsc::Receiver<(u64, u64, u64, [u8; 32])>,
	pub sync_service: Arc<S>,
}

pub(crate) struct WorkerParams<B: Block, A, BN, C, R, BE> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_engine: GossipEngine<B>,
	pub kv: KeystorePtr,
	pub accountid: PhantomData<A>,
	pub _block_number: PhantomData<BN>,
	pub sign_data_receiver: mpsc::Receiver<(u64, u64, u64, [u8; 32])>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_timeworker_gadget<B, A, BN, C, R, BE, N, S>(
	timeworker_params: TimeWorkerParams<B, A, BN, C, R, BE, N, S>,
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
	let TimeWorkerParams {
		client,
		backend,
		runtime,
		gossip_network,
		kv,
		_block,
		sign_data_receiver,
		accountid,
		_block_number,
		sync_service,
	} = timeworker_params;
	let gossip_engine = GossipEngine::new(
		gossip_network,
		sync_service,
		gossip_protocol_name(),
		Arc::new(GossipValidator::new()),
		None,
	);

	let worker_params = WorkerParams {
		client,
		backend,
		runtime,
		gossip_engine,
		kv,
		sign_data_receiver,
		accountid,
		_block_number,
	};
	let mut worker = worker::TimeWorker::<_, _, _, _, _, _>::new(worker_params);
	worker.run().await
}
