#![allow(clippy::type_complexity)]

pub mod worker;

use futures::channel::mpsc::Sender;
use log::*;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "connector-worker";

/// Set of properties we need to run our gadget
pub struct ConnectorWorkerParams<B: Block, A, R, BE>
where
	B: Block,
	A: codec::Codec,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B, A>,
{
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub kv: KeystorePtr,
	pub _block: PhantomData<B>,
	pub accountid: PhantomData<A>,
	pub sign_data_sender: Sender<(u64, [u8; 32])>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

pub(crate) struct WorkerParams<B, A, R, BE> {
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	accountid: PhantomData<A>,
	pub sign_data_sender: Sender<(u64, [u8; 32])>,
	kv: KeystorePtr,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
}

pub async fn start_connectorworker_gadget<B, A, R, BE>(
	connectorworker_params: ConnectorWorkerParams<B, A, R, BE>,
) where
	B: Block,
	A: codec::Codec + 'static,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B, A>,
{
	debug!(target: TW_LOG, "Starting ConnectorWorker gadget");
	let ConnectorWorkerParams {
		runtime,
		kv,
		sign_data_sender,
		backend,
		_block,
		accountid: _,
		connector_url,
		connector_blockchain,
		connector_network,
	} = connectorworker_params;

	let worker_params = WorkerParams {
		runtime,
		kv,
		backend,
		_block,
		accountid: PhantomData,
		sign_data_sender,
		connector_url,
		connector_blockchain,
		connector_network,
	};
	let mut worker = worker::ConnectorWorker::<_, _, _, _>::new(worker_params);
	worker.run().await
}
