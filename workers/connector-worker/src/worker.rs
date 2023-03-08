#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use futures::channel::mpsc::Sender;
use log::warn;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, str::FromStr, sync::Arc, thread};
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;
use web3::{
	futures::{future, StreamExt},
	types::{Address, BlockId, BlockNumber, FilterBuilder, U64},
};
use worker_aurora::{self, establish_connection, get_on_chain_data};

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

	pub fn get_swap_data_from_db() -> Vec<Vec<u8>> {
		let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		let mut pg_conn = establish_connection(Some(conn_url));
		let tasks_from_db = get_on_chain_data(&mut pg_conn, 10);

		let mut tasks_from_db_bytes = vec![vec![]];
		for task in tasks_from_db.iter() {
			let task_in_bytes = serialize(task).unwrap();
			tasks_from_db_bytes.push(task_in_bytes);
		}
		return tasks_from_db_bytes;
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

		let filter = FilterBuilder::default()
			.from_block(BlockNumber::Number(U64::from(latest_block.number.unwrap())))
			.address(vec![Address::from_str("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984").unwrap()])
			.build();

		let sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

		let mut log_vec = vec![vec![]];
		let result = log_vec.clone();

		thread::spawn(move || {
			sub.for_each(move |log| {
				match log {
					Ok(log) => {
						let log_in_bytes = serialize(&log).unwrap();
						log_vec.push(log_in_bytes);
					},
					Err(e) => log::info!("getting error {:?}", e),
				}
				future::ready(())
			})
		});
		return result;
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(3);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				let tasks_in_byte = Self::get_swap_data_from_db();
				if tasks_in_byte.len() > 0 {
					let result =
						sign_data_sender_clone.lock().await.try_send((1, tasks_in_byte[0].clone()));
					match result {
						Ok(_) => warn!("sign_data_sender_clone ok"),
						Err(_) => warn!("sign_data_sender_clone err"),
					}
				}

				let events_from_eth = Self::get_latest_block().await;
				if events_from_eth.len() > 0 {
					for event in events_from_eth.iter() {
						let result =
							sign_data_sender_clone.lock().await.try_send((1, event.clone()));
						match result {
							Ok(_) => warn!("sign_data_sender_clone ok"),
							Err(_) => warn!("sign_data_sender_clone err"),
						}
					}
				}
				thread::sleep(delay);
			}
		}
	}
}
