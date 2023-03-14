#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use futures::channel::mpsc::Sender;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{ marker::PhantomData, sync::Arc, thread};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time};

// use worker_aurora::{self, establish_connection, get_on_chain_data};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, R> {
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
}

impl<B, R> TaskExecutor<B, R>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	// R::Api: GetStoreTask<B>,
	// R::Api: GetTaskMetaData<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, R>) -> Self {
		let WorkerParams {
			runtime,
			sign_data_sender,
			kv,
			_block,
		} = worker_params;

		TaskExecutor {
			runtime,
			sign_data_sender,
			kv,
			_block: PhantomData,
		}
	}

	
	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(3);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {

				log::info!("\n\n\n\n new task executor \n\n\n\n");
                
				thread::sleep(delay);
			}
		}
	}
}
