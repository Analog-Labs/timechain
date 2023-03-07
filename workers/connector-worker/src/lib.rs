#![allow(clippy::type_complexity)]

pub mod worker;

use futures::channel::mpsc::Sender;
use log::*;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_worker::kv::TimeKeyvault;
// use storage_primitives::{GetStoreTask, GetTaskMetaData};
use tokio::sync::Mutex;

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
pub struct ConnectorWorkerParams<B: Block, R>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	// R::Api: GetStoreTask<B>,
	// R::Api: GetTaskMetaData<B>,
{
	pub runtime: Arc<R>,
	pub kv: TimeKeyvault,
	pub _block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, Vec<u8>)>>>,
}

pub(crate) struct WorkerParams<B, R> {
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, Vec<u8>)>>>,
	kv: TimeKeyvault,
}

/// Start the Timeworker gadget.
///
/// This is a thin shim around running and awaiting a time worker.
pub async fn start_connectorworker_gadget<B, R>(connectorworker_params: ConnectorWorkerParams<B, R>)
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	// R::Api: GetStoreTask<B>,
	// R::Api: GetTaskMetaData<B>,
{
	debug!(target: TW_LOG, "Starting ConnectorWorker gadget");
	let ConnectorWorkerParams {
		runtime,
		kv,
		sign_data_sender,
		_block,
	} = connectorworker_params;

	// let sync_oracle = gossip_network.clone();
	// let gossip_validator = Arc::new(GossipValidator::new());
	// let gossip_engine =
	// 	GossipEngine::new(gossip_network, gossip_protocol_name(), gossip_validator.clone(), None);

	let worker_params = WorkerParams {
		runtime,
		kv,
		_block,
		sign_data_sender,
	};
	let mut worker = worker::ConnectorWorker::<_, _>::new(worker_params);
	worker.run().await
}
