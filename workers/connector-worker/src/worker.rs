#![allow(clippy::type_complexity)]
use crate::{Client, WorkerParams};
use connector::ethereum::SwapToken;
use core::time;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, thread};
use storage_primitives::{GetStoreTask, GetTaskMetaData};
use web3::transports::Http;

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct ConnectorWorker<B: Block, C, R, BE> {
	pub(crate) client: Arc<C>,
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<tokio::sync::Mutex<futures_channel::mpsc::Sender<Vec<i32>>>>,
}

impl<B, C, R, BE> ConnectorWorker<B, C, R, BE>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: GetStoreTask<B>,
	R::Api: GetTaskMetaData<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, C, R, BE>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			sign_data_sender,
			_block,
		} = worker_params;

		ConnectorWorker {
			client,
			backend,
			runtime,
			sign_data_sender,
			_block: PhantomData,
		}
	}

	pub(crate) async fn run(&mut self) {
		let a = self.sign_data_sender.clone();

		//Connector for swap price
		let end_point = Http::new("http://127.0.0.1:8545");
		let abi = "./contracts/artifacts/contracts/swap_price.sol/TokenSwap.json";
		let exchange_address = "0x5FbDB2315678afecb367f032d93F642f64180aa3";
		let delay = time::Duration::from_secs(3);
		loop {
			let swap_result = SwapToken::swap_price(
				&web3::Web3::new(end_point.clone().unwrap()),
				abi,
				exchange_address,
				"getAmountsOut",
				std::string::String::from("1"),
			)
			.await
			.unwrap();
			a.lock().await.try_send(swap_result);
			thread::sleep(delay);
		}
	}
}
