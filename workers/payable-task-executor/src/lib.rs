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
pub const TW_LOG: &str = "payable-task-executor";

/// Set of properties we need to run our gadget
pub struct PayableTaskExecutorParams<B: Block, A, BN, R, BE>
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
	pub tx_data_sender: Sender<Vec<u8>>,
	pub gossip_data_sender: Sender<Vec<u8>>,
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
	pub tx_data_sender: Sender<Vec<u8>>,
	pub gossip_data_sender: Sender<Vec<u8>>,
	kv: KeystorePtr,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

/// Start the payable task Executor gadget.
///
/// This is a thin shim around running and awaiting a payable task Executor.
pub async fn start_payabletaskexecutor_gadget<B, A, BN, R, BE>(
	payabletaskexecutor_params: PayableTaskExecutorParams<B, A, BN, R, BE>,
) where
	B: Block,
	A: codec::Codec + 'static,
	BN: codec::Codec + 'static,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A, BN>,
{
	debug!(target: TW_LOG, "Starting payable-task-executor gadget");
	let PayableTaskExecutorParams {
		backend,
		runtime,
		kv,
		tx_data_sender,
		gossip_data_sender,
		_block,
		accountid,
		_block_number,
		connector_url,
		connector_blockchain,
		connector_network,
	} = payabletaskexecutor_params;

	let worker_params = WorkerParams {
		backend,
		runtime,
		kv,
		_block,
		tx_data_sender,
		gossip_data_sender,
		accountid,
		_block_number,
		connector_url,
		connector_blockchain,
		connector_network,
	};
	let mut worker = worker::PayableTaskExecutor::<_, _, _, _, _>::new(worker_params);
	worker.run().await
}
