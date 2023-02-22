#![allow(clippy::type_complexity)]
use crate::{WorkerParams};
use core::time;
use log::warn;
use worker_aurora::{self, get_on_chain_data, establish_connection};
use futures::channel::mpsc::Sender;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, thread};
// use storage_primitives::{GetStoreTask, GetTaskMetaData};
use time_worker::kv::{TimeKeyvault};
use tokio::sync::Mutex;

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct ConnectorWorker<B: Block, R> {
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, Vec<u8>)>>>,
	kv: TimeKeyvault,
}

impl<B, R> ConnectorWorker<B, R>
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

		ConnectorWorker {
			runtime,
			sign_data_sender,
			kv,
			_block: PhantomData,
		}
	}

	pub fn get_swap_data_from_db() -> Vec<u8> {
		let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		let mut pg_conn = establish_connection(Some(conn_url));
		let data = get_on_chain_data(&mut pg_conn, 0);

		log::info!("data from db = {:?}",data);

		return vec![1, 2];
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(3);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				let result = sign_data_sender_clone
					.lock()
					.await
					.try_send((1, Self::get_swap_data_from_db()));
				match result {
					Ok(_) => warn!("+++++++++++++ sign_data_sender_clone ok"),
					Err(_) => warn!("+++++++++++++ sign_data_sender_clone err"),
				}
					// .unwrap();
				thread::sleep(delay);
			}
		}
	}
}
