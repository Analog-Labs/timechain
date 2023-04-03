#![allow(clippy::type_complexity)]

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
pub struct TaskExecutorParams<B: Block, R, BE>
where
	B: Block,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
{
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub kv: TimeKeyvault,
	pub _block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
}

pub(crate) struct WorkerParams<B, R, BE> {
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
}

/// Start the task Executor gadget.
///
/// This is a thin shim around running and awaiting a task Executor.
pub async fn start_taskexecutor_gadget<B, R, BE>(taskexecutor_params: TaskExecutorParams<B, R, BE>)
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B>,
{
	debug!(target: TW_LOG, "Starting task-executor gadget");
	let TaskExecutorParams {
		backend,
		runtime,
		kv,
		sign_data_sender,
		_block,
	} = taskexecutor_params;

	let worker_params = WorkerParams {
		backend,
		runtime,
		kv,
		_block,
		sign_data_sender,
	};
	let mut worker = worker::TaskExecutor::<_, _, _>::new(worker_params);
	worker.run().await
}
