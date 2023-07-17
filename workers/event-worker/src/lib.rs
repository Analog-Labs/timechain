#![allow(clippy::type_complexity)]

pub mod worker;

use futures::channel::mpsc::{Receiver, Sender};
use log::*;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::{sharding::ShardId, TimeApi};

/// Constant to indicate target for logging
pub const TW_LOG: &str = "event-worker";

/// Set of properties we need to run our gadget
pub struct EventWorkerParams<B: Block, A, BN, R, BE>
where
	B: Block,
	A: codec::Codec,
	BN: codec::Codec,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B, A, BN>,
{
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub kv: KeystorePtr,
	pub _block: PhantomData<B>,
	pub accountid: PhantomData<A>,
	pub _block_number: PhantomData<BN>,
	pub sign_data_sender: Sender<(ShardId, u64, u64, [u8; 32])>,
	pub tx_data_receiver: Receiver<Vec<u8>>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

pub(crate) struct WorkerParams<B, A, BN, R, BE> {
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	accountid: PhantomData<A>,
	_block_number: PhantomData<BN>,
	pub sign_data_sender: Sender<(ShardId, u64, u64, [u8; 32])>,
	pub tx_data_receiver: Receiver<Vec<u8>>,
	kv: KeystorePtr,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
}

pub async fn start_eventworker_gadget<B, A, BN, R, BE>(
	eventworker_params: EventWorkerParams<B, A, BN, R, BE>,
) where
	B: Block,
	A: codec::Codec + 'static,
	BN: codec::Codec + 'static,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B, A, BN>,
{
	debug!(target: TW_LOG, "Starting EventWorker gadget");
	let EventWorkerParams {
		runtime,
		kv,
		sign_data_sender,
		tx_data_receiver,
		backend,
		_block,
		accountid,
		_block_number,
		connector_url,
		connector_blockchain,
		connector_network,
	} = eventworker_params;

	let worker_params = WorkerParams {
		runtime,
		kv,
		backend,
		_block,
		accountid,
		_block_number,
		sign_data_sender,
		tx_data_receiver,
		connector_url,
		connector_blockchain,
		connector_network,
	};
	let mut worker = worker::EventWorker::<_, _, _, _, _>::new(worker_params);
	worker.run().await
}
