#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use futures::channel::mpsc::Sender;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_runtime::{generic::BlockId, traits::Block};
use std::{marker::PhantomData, sync::Arc, thread};
use time_primitives::TimeApi;
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time};

// use worker_aurora::{self, establish_connection, get_on_chain_data};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
}

impl<B, R, BE> TaskExecutor<B, R, BE>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, R, BE>) -> Self {
		let WorkerParams {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block,
		} = worker_params;

		TaskExecutor {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block: PhantomData,
		}
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(10);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				let at = self.backend.blockchain().last_finalized().unwrap();
				let at = BlockId::Hash(at);

				if let Ok(metadata) = self.runtime.runtime_api().get_task_metadata(&at) {
					log::info!("New task metadata: {:?}", metadata);
				} else {
					log::error!("Failed to get task metadata for block {:?}", at);
				}
				thread::sleep(delay);
			}
		}
	}
}
