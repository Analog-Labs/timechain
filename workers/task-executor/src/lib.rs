use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use time_primitives::{PublicKey, TaskSpawner, TasksApi};

mod worker;

pub use crate::worker::{Task, TaskExecutor, TaskSpawnerParams};

#[cfg(test)]
mod tests;

/// Constant to indicate target for logging
pub const TW_LOG: &str = "task-executor";

/// Set of properties we need to run our gadget
#[derive(Clone)]
pub struct TaskExecutorParams<B: Block, R, T>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	R::Api: TasksApi<B>,
	T: TaskSpawner,
{
	pub _block: PhantomData<B>,
	pub runtime: Arc<R>,
	pub kv: KeystorePtr,
	pub public_key: PublicKey,
	pub offchain_tx_pool_factory: OffchainTransactionPoolFactory<B>,
	pub task_spawner: T,
}
