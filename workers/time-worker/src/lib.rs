#![allow(clippy::type_complexity)]
#![feature(type_alias_impl_trait)]
pub mod communication;
pub mod inherents;
pub mod kv;
pub mod traits;
mod tss_event_handler_helper;
pub mod worker;

#[cfg(test)]
mod tests;
use crate::{
	communication::{time_protocol_name::gossip_protocol_name, validator::GossipValidator},
	kv::TimeKeyvault,
};
use futures::channel::mpsc::Receiver as FutReceiver;
use log::*;
use sc_client_api::Backend;
use sc_network_gossip::{GossipEngine, Network as GossipNetwork, Syncing as GossipSyncing};
use sp_api::ProvideRuntimeApi;
use sp_consensus::SyncOracle;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;
use tokio::{self, sync::Mutex as TokioMutex};
use traits::Client;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "⌛time-worker";

/// Set of properties we need to run our gadget
pub struct TimeWorkerParams<B: Block, C, R, BE, N, S>
where
	B: Block + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	S: GossipSyncing<B> + SyncOracle + 'static,
{
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_network: N,
	pub kv: TimeKeyvault,
	pub _block: PhantomData<B>,
	pub sign_data_receiver: Arc<TokioMutex<FutReceiver<(u64, [u8; 32])>>>,
	pub sync_service: Arc<S>,
}

pub(crate) struct WorkerParams<B: Block, C, R, BE> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub gossip_engine: GossipEngine<B>,
	pub gossip_validator: Arc<GossipValidator<B>>,
	pub kv: TimeKeyvault,
	pub sign_data_receiver: Arc<TokioMutex<FutReceiver<(u64, [u8; 32])>>>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_timeworker_gadget<B, C, R, BE, N, S>(
	timeworker_params: TimeWorkerParams<B, C, R, BE, N, S>,
) where
	B: Block + 'static,
	BE: Backend<B> + 'static,
	C: Client<B, BE> + 'static,
	R: ProvideRuntimeApi<B> + 'static,
	R::Api: TimeApi<B>,
	S: GossipSyncing<B> + SyncOracle + 'static,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
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
	};
	let mut worker = worker::TimeWorker::<_, _, _, _>::new(worker_params);
	worker.run().await
}
