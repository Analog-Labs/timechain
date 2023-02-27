#![allow(clippy::type_complexity)]
use crate::WorkerParams;
// use connector::get_latest_block;
use core::time;
use futures::channel::mpsc::Sender;
use log::warn;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, thread};
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;
use worker_aurora::{self, establish_connection, get_on_chain_data};

use bincode::{deserialize, serialize};
use std::str::FromStr;
use web3::{
	futures::{future, StreamExt},
	types::{Address, BlockId, BlockNumber, FilterBuilder, U64},
};

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
		// let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		// let mut pg_conn = establish_connection(Some(conn_url));
		// let data = get_on_chain_data(&mut pg_conn, 0);

		// log::info!("data from db = {:?}",data);

		return vec![1, 2];
	}

	pub async fn get_latest_block() -> Vec<u8> {
		let websocket = web3::transports::WebSocket::new(
			"wss://goerli.infura.io/ws/v3/0e188b05227b4af7a7a4a93a6282b0c8",
		)
		.await
		.unwrap();
		let web3 = web3::Web3::new(websocket);

		let latest_block =
			web3.eth().block(BlockId::Number(BlockNumber::Latest)).await.unwrap().unwrap();

		println!(
			"\nblock number {} \nnumber of transactions: {} \ndifficulty {}\n",
			latest_block.number.unwrap(),
			&latest_block.transactions.len(),
			&latest_block.total_difficulty.unwrap()
		);

		let filter = FilterBuilder::default()
			.from_block(BlockNumber::Number(U64::from(latest_block.number.unwrap())))
			.address(vec![Address::from_str("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984").unwrap()])
			.build();

		let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

		let mut a = vec![];

		// sub.for_each(|log| {
		// 	match log {
		// 		Ok(log) => {
		// 			let my_struct_bytes = serialize(&log).unwrap();
		// 			println!("==>{:?}", my_struct_bytes);
		// 			a = my_struct_bytes;
		// 		},
		// 		Err(e) => log::info!("getting error {:?}", e),
		// 	}
		// 	future::ready(())
		// })
		// .await;
	println!("\n\n\nhre===> {:?}\n\n\n\n",sub);
		return a;
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

				let result = sign_data_sender_clone
					.lock()
					.await
					.try_send((1, Self::get_latest_block().await));
				match result {
					Ok(_) => warn!("+++++++++++++ sign_data_sender_clone ok"),
					Err(_) => warn!("+++++++++++++ sign_data_sender_clone err"),
				}
				thread::sleep(delay);
			}
		}
	}
}

// #[ignore]
// #[actix_rt::test]
// async fn get_latest_block_test() {
// 	println!("{:?}", ConnectorWorker::get_latest_block().await);
// }
