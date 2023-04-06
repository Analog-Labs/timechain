#![allow(clippy::type_complexity)]

pub mod worker;

use futures::channel::mpsc::Sender;
use log::*;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "connector-worker";

/// Set of properties we need to run our gadget
pub struct ConnectorWorkerParams<B: Block, R>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
{
	pub runtime: Arc<R>,
	pub kv: TimeKeyvault,
	pub _block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	pub connector_url: String,
	pub connector_blockchain: String,
	pub connector_network: String,
}

pub(crate) struct WorkerParams<B, R> {
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
	pub connector_url: String,
	pub connector_blockchain: String,
	pub connector_network: String,
}

pub async fn start_connectorworker_gadget<B, R>(connectorworker_params: ConnectorWorkerParams<B, R>)
where
	B: Block,
	R: ProvideRuntimeApi<B>,
{
	debug!(target: TW_LOG, "Starting ConnectorWorker gadget");
	let ConnectorWorkerParams {
		runtime,
		kv,
		sign_data_sender,
		_block,
		connector_url,
		connector_blockchain,
		connector_network,
	} = connectorworker_params;

	let worker_params = WorkerParams {
		runtime,
		kv,
		_block,
		sign_data_sender,
		connector_url,
		connector_blockchain,
		connector_network,
	};
	let mut worker = worker::ConnectorWorker::<_, _>::new(worker_params);
	worker.run().await
}
