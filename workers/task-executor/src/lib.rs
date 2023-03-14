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


/// Constant to indicate target for logging
pub const TW_LOG: &str = "task-executor";

/// Set of properties we need to run our gadget
pub struct TaskExecutorParams<B: Block, R>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	// R::Api: GetStoreTask<B>,
	// R::Api: GetTaskMetaData<B>,
{
	pub runtime: Arc<R>,
	pub kv: TimeKeyvault,
	pub _block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
}

pub(crate) struct WorkerParams<B, R> {
	pub runtime: Arc<R>,
	_block: PhantomData<B>,
	pub sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
}

/// Start the task Executor gadget.
///
/// This is a thin shim around running and awaiting a task Executor.
pub async fn start_taskexecutor_gadget<B, R>(taskexecutor_params: TaskExecutorParams<B, R>)
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	// R::Api: GetStoreTask<B>,
	// R::Api: GetTaskMetaData<B>,
{
	debug!(target: TW_LOG, "Starting task-executor gadget");
	let TaskExecutorParams {
		runtime,
		kv,
		sign_data_sender,
		_block,
	} = taskexecutor_params;

	
	let worker_params = WorkerParams {
		runtime,
		kv,
		_block,
		sign_data_sender,
	};
	let mut worker = worker::TaskExecutor::<_, _>::new(worker_params);
	worker.run().await
}
