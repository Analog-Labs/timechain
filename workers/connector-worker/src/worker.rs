#![allow(clippy::type_complexity)]
use crate::WorkerParams;
// use connector::get_latest_block;
use core::time;
use diesel::prelude::*;
use diesel::sql_query;
use futures::channel::mpsc::Sender;
use log::warn;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{env, marker::PhantomData, sync::Arc, thread};
use worker_aurora::{self, establish_connection, get_on_chain_data};
// use storage_primitives::{GetStoreTask, GetTaskMetaData};
use bincode::serialize;
use std::str::FromStr;
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;
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
	fn establish() -> PgConnection {
		let database_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		PgConnection::establish(&database_url)
			.expect(&format!("Error connecting to {}", database_url))
	}
	async fn listen_for_table_updates() {
		// let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		// let pg_conn = PgConnection::establish(conn_url);
		let conn = Self::establish();
		// Listen for updates to the `users` table
		let stmt = sql_query("LISTEN table_update");
		diesel::sql_query("LISTEN table_update").execute(&mut conn).unwrap();

		// Wait for notifications
		loop {
			let notification = diesel::sql_query("SELECT * FROM pg_notification")
				.get_result::<(String, String)>(&mut conn);

			match notification {
				Ok(n) => {
					if n.0 == "table_update" {
						// Handle the update here
						println!("Table update received: {}", n.1);
					}
				},
				Err(_) => continue,
			}
		}
	}
	pub fn get_swap_data_from_db() -> Vec<u8> {
		let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		let mut pg_conn = establish_connection(Some(conn_url));
		let data = get_on_chain_data(&mut pg_conn, 10);

		log::info!("data from db = {:?}", data);

		return vec![1, 2];
	}

	pub async fn get_latest_block() -> Vec<Vec<u8>> {
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

		let mut log_vec = vec![vec![]];
		let result = log_vec.clone();

		// thread::spawn(move || {
		// 	sub.for_each(move |log| {
		// 		match log {
		// 			Ok(log) => {
		// 				let my_struct_bytes = serialize(&log).unwrap();
		// 				println!("==>{:?}", my_struct_bytes);
		// 				log_vec.push(my_struct_bytes);
		// 			},
		// 			Err(e) => log::info!("getting error {:?}", e),
		// 		}
		// 		future::ready(())
		// 	})
		// });
		return result;
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(3);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				Self::get_swap_data_from_db();
				// let result = sign_data_sender_clone
				// 	.lock()
				// 	.await
				// 	.try_send((1, Self::get_swap_data_from_db()));
				// match result {
				// 	Ok(_) => warn!("+++++++++++++ sign_data_sender_clone ok"),
				// 	Err(_) => warn!("+++++++++++++ sign_data_sender_clone err"),
				// }

				// let x = Self::get_latest_block().await;
				// if x.len() > 0 {
				// 	let result = sign_data_sender_clone.lock().await.try_send((1, x[0].clone()));
				// 	match result {
				// 		Ok(_) => warn!("=> sign_data_sender_clone ok"),
				// 		Err(_) => warn!("=> sign_data_sender_clone err"),
				// 	}
				// }
				thread::sleep(delay);
			}
		}
	}
}

// #[ignore]
// #[test]
// fn get_latest_block_test() {
// 	let x =  ConnectorWorker::get_latest_block();

// 	println!("{:?}",x);
// }
