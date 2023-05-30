#![allow(clippy::type_complexity)]
mod communication;
mod inherents;
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
pub const TW_LOG: &str = "⌛time-worker";

/// Set of properties we need to run our gadget
pub struct TimeWorkerParams<B: Block, A, C, R, BE, N, S>
where
	B: Block + 'static,
	A: sp_runtime::codec::Codec + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B, A>,
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
	pub sign_data_receiver: mpsc::Receiver<(u64, [u8; 32])>,
	pub tx_data_sender: mpsc::Sender<Vec<u8>>,
	pub gossip_data_receiver: mpsc::Receiver<Vec<u8>>,
	pub sync_service: Arc<S>,
}

pub(crate) struct WorkerParams<B: Block, A, C, R, BE> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_engine: GossipEngine<B>,
	pub gossip_validator: Arc<GossipValidator<B>>,
	pub kv: KeystorePtr,
	pub accountid: PhantomData<A>,
	pub sign_data_receiver: mpsc::Receiver<(u64, [u8; 32])>,
	pub tx_data_sender: mpsc::Sender<Vec<u8>>,
	pub gossip_data_receiver: mpsc::Receiver<Vec<u8>>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_timeworker_gadget<B, A, C, R, BE, N, S>(
	timeworker_params: TimeWorkerParams<B, A, C, R, BE, N, S>,
) where
	B: Block + 'static,
	A: sp_runtime::codec::Codec + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B, A>,
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
		tx_data_sender,
		gossip_data_receiver,
		accountid: _,
		sync_service,
	} = timeworker_params;
	let gossip_validator = Arc::new(GossipValidator::new());
	let gossip_engine = GossipEngine::new(
		gossip_network,
		sync_service,
		gossip_protocol_name(),
		gossip_validator.clone(),
		None,
	);

	let worker_params = WorkerParams {
		client,
		backend,
		runtime,
		gossip_validator,
		gossip_engine,
		kv,
		sign_data_receiver,
		tx_data_sender,
		gossip_data_receiver,
		accountid: PhantomData,
	};
	let mut worker = worker::TimeWorker::<_, _, _, _, _>::new(worker_params);
	worker.run().await
}
