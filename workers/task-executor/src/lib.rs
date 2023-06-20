use crate::worker::TaskExecutor;
use futures::channel::mpsc::Sender;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::TimeApi;

mod task_schedule;
mod worker;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "task-executor";

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<B: Block, A, R, BE>
where
	B: Block,
	A: codec::Codec + Clone,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	R::Api: TimeApi<B, A>,
{
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub kv: KeystorePtr,
	pub _block: PhantomData<B>,
	pub accountid: PhantomData<A>,
	pub sign_data_sender: Sender<(u64, u64, u64, [u8; 32])>,
	pub connector_url: Option<String>,
	pub connector_blockchain: Option<String>,
	pub connector_network: Option<String>,
}

/// Start the task Executor gadget.
///
/// This is a thin shim around running and awaiting a task Executor.
pub async fn start_taskexecutor_gadget<B, A, R, BE>(params: TaskExecutorParams<B, A, R, BE>)
where
	B: Block,
	A: codec::Codec + Clone + 'static + std::marker::Send + std::marker::Sync,
	R: ProvideRuntimeApi<B> + std::marker::Sync + std::marker::Send + 'static,
	BE: Backend<B> + 'static,
	R::Api: TimeApi<B, A>,
{
	log::debug!(target: TW_LOG, "Starting task-executor gadget");
	// match Arc::try_unwrap(params) {
	// 	Ok(params) => {
	let mut worker = TaskExecutor::new(params).await.unwrap();
	worker.run().await;
	// 	},
	// 	Err(_) => log::warn!("Cannot unwrap Arc: "),
	// }
}
