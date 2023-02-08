#![allow(clippy::type_complexity)]

pub mod communication;
pub mod inherents;
pub mod kv;
pub mod traits;
pub mod worker;

use log::*;
use sc_client_api::Backend;

use sp_api::ProvideRuntimeApi;

use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use storage_primitives::{GetStoreTask, GetTaskMetaData};
use traits::Client;

/*gossip_engine: Arc::new(Mutex::new(GossipEngine::new(
network.clone(),
gossip_protocol_name(),
gossip_validator.clone(),
None,
))),
*/
/// Constant to indicate target for logging
pub const TW_LOG: &str = "⌛time-worker";

/// Set of properties we need to run our gadget
pub struct ConnectorWorkerParams<B: Block, C, R, BE>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: GetStoreTask<B>,
	R::Api: GetTaskMetaData<B>,
{
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub _block: PhantomData<B>,
	pub sign_data_sender: Arc<tokio::sync::Mutex<futures_channel::mpsc::Sender<Vec<i32>>>>,
}

pub(crate) struct WorkerParams<B, C, R, BE> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	pub sign_data_sender: Arc<tokio::sync::Mutex<futures_channel::mpsc::Sender<Vec<i32>>>>,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_connectorworker_gadget<B, C, R, BE>(
	connectorworker_params: ConnectorWorkerParams<B, C, R, BE>,
) where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: GetStoreTask<B>,
	R::Api: GetTaskMetaData<B>,
{
	debug!(target: TW_LOG, "Starting ConnectorWorker gadget");
	let ConnectorWorkerParams {
		client,
		backend,
		runtime,
		sign_data_sender,
		_block,
	} = connectorworker_params;

	// let sync_oracle = gossip_network.clone();
	// let gossip_validator = Arc::new(GossipValidator::new());
	// let gossip_engine =
	// 	GossipEngine::new(gossip_network, gossip_protocol_name(), gossip_validator.clone(), None);

	let worker_params = WorkerParams {
		client,
		backend,
		runtime,
		_block,
		sign_data_sender,
	};
	let mut worker = worker::ConnectorWorker::<_, _, _, _>::new(worker_params);
	worker.run().await
}
