#![allow(clippy::type_complexity)]

mod schema;
pub mod worker;

use futures::channel::mpsc::Sender;
use log::*;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "task-executor";

/// Set of properties we need to run our gadget
pub struct TaskExecutorParams<B: Block, A, R, BE>
where
	B: Block,
	A: codec::Codec,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B, A>,
{
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub kv: TimeKeyvault,
	pub _block: PhantomData<B>,
	pub accountid: PhantomData<A>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

pub(crate) struct WorkerParams<B, A, R, BE> {
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	accountid: PhantomData<A>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

/// Start the task Executor gadget.
///
/// This is a thin shim around running and awaiting a task Executor.
pub async fn start_taskexecutor_gadget<B, A, R, BE>(
	taskexecutor_params: TaskExecutorParams<B, A, R, BE>,
) where
	B: Block,
	A: codec::Codec + 'static,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A>,
{
	debug!(target: TW_LOG, "Starting task-executor gadget");
	let TaskExecutorParams {
		backend,
		runtime,
		kv,
		sign_data_sender,
		_block,
		accountid: _,
		connector_url,
		connector_blockchain,
		connector_network,
	} = taskexecutor_params;

	let worker_params = WorkerParams {
		backend,
		runtime,
		kv,
		_block,
		sign_data_sender,
		accountid: PhantomData,
		connector_url,
		connector_blockchain,
		connector_network,
	};
	let mut worker = worker::TaskExecutor::<_, _, _, _>::new(worker_params);
	worker.run().await
}
