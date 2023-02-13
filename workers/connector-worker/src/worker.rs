#![allow(clippy::type_complexity)]
use crate::WorkerParams;
// use connector::ethereum::SwapToken;
use core::time;
use futures::channel::mpsc::Sender;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, thread};
use storage_primitives::{GetStoreTask, GetTaskMetaData};
use tokio::sync::Mutex;
// use web3::transports::Http;

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct ConnectorWorker<B: Block, R> {
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, Vec<u8>)>>>,
}

impl<B, R> ConnectorWorker<B, R>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	R::Api: GetStoreTask<B>,
	R::Api: GetTaskMetaData<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, R>) -> Self {
		let WorkerParams {
			runtime,
			sign_data_sender,
			_block,
		} = worker_params;

		ConnectorWorker {
			runtime,
			sign_data_sender,
			_block: PhantomData,
		}
	}

	pub fn get_swap_data_from_db() -> Vec<u8> {
		return vec![1, 2];
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(3);
		loop {
			sign_data_sender_clone
				.lock()
				.await
				.try_send((1, Self::get_swap_data_from_db()))
				.unwrap();
			thread::sleep(delay);
		}
	}
}
