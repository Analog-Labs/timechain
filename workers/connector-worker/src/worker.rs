#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use futures::channel::mpsc::Sender;
use ink::env::hash;
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
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
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

	pub fn hash_keccak_256(input: &[u8]) -> [u8; 32] {
		let mut output = <hash::Keccak256 as hash::HashOutput>::Type::default();
		ink::env::hash_bytes::<hash::Keccak256>(input, &mut output);
		output
	}

	pub fn get_swap_data_from_db() -> Vec<[u8; 32]> {
		let conn_url = "postgresql://localhost/timechain?user=postgres&password=postgres";
		let mut pg_conn = establish_connection(Some(conn_url));
		let tasks_from_db = get_on_chain_data(&mut pg_conn, 10);

		let mut tasks_from_db_bytes: Vec<[u8; 32]> = Vec::new();
		for task in tasks_from_db.iter() {
			let task_in_bytes: &[u8] = &serialize(task).unwrap();
			tasks_from_db_bytes.push(Self::hash_keccak_256(task_in_bytes));
		}
		return tasks_from_db_bytes;
	}

	pub async fn get_latest_block() -> Vec<[u8; 32]> {
		let websocket = web3::transports::WebSocket::new(
			"wss://goerli.infura.io/ws/v3/<ADD-your-infura-id>",
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

		let mut log_vec: Vec<[u8; 32]> = Vec::new();
		let result = log_vec.clone();

		thread::spawn(move || {
			sub.for_each(move |log| {
				match log {
					Ok(log) => {
						let log_in_bytes: &[u8] = &serialize(&log).unwrap();
						log_vec.push(Self::hash_keccak_256(log_in_bytes));
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
					for task in tasks_in_byte.iter() {
						let result = sign_data_sender_clone.lock().await.try_send((1, *task));
						match result {
							Ok(_) => warn!("sign_data_sender_clone ok"),
							Err(_) => warn!("sign_data_sender_clone err"),
						}
					}
				}

				let events_from_eth = Self::get_latest_block().await;
				if events_from_eth.len() > 0 {
					for event in events_from_eth.iter() {
						let result = sign_data_sender_clone.lock().await.try_send((1, *event));
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
