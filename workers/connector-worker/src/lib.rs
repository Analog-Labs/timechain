use crate::worker::ConnectorWorker;
use futures::channel::mpsc::Sender;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;

mod worker;

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

pub async fn start_connectorworker_gadget<B, A, R, BE>(params: ConnectorWorkerParams<B, A, R, BE>)
where
	B: Block,
	A: codec::Codec + 'static,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A>,
{
	log::debug!(target: TW_LOG, "Starting connector worker gadget");
	let mut worker = ConnectorWorker::new(params).await.unwrap();
	worker.run().await
}
