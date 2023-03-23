#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use ink::env::hash;
use log::warn;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{env, marker::PhantomData, str::FromStr, sync::Arc, thread};
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;
use web3::{
	futures::StreamExt,
	types::{Address, BlockId, BlockNumber, FilterBuilder},
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

		let mut tasks_from_db_bytes: Vec<[u8; 32]> = Vec::new();
		if let Ok(mut pg_conn) = establish_connection(Some(conn_url)) {
			if let Ok(tasks_from_db) = get_on_chain_data(&mut pg_conn, 10) {
				for task in tasks_from_db.iter() {
					if let Ok(task_in_bytes) = serialize(task) {
						tasks_from_db_bytes.push(Self::hash_keccak_256(&task_in_bytes));
					} else {
						log::info!("Failed to serialize task: {:?}", task);
					}
				}
			}
		}
		tasks_from_db_bytes
	}

	pub async fn get_latest_block_event(&self) {
		dotenv().ok();

		let infura_url = env::var("INFURA_URL").expect("INFURA_URL must be set");
		let websocket_result = web3::transports::WebSocket::new(&infura_url).await;
		let websocket = match websocket_result {
			Ok(websocket) => websocket,
			Err(_) => web3::transports::WebSocket::new(&infura_url)
				.await
				.expect("Failed to create default websocket"),
		};
		let web3 = web3::Web3::new(websocket);
		if let Ok(Some(latest_block)) = web3.eth().block(BlockId::Number(BlockNumber::Latest)).await
		{
			let filter = FilterBuilder::default()
				.from_block(match latest_block.number {
					Some(n) => BlockNumber::Number(n),
					None => {
						log::warn!("latest_block.number is None, setting from_block to earliest");
						BlockNumber::Earliest
					},
				})
				.address(vec![if let Ok(address) =
					Address::from_str("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984")
				{
					address
				} else {
					panic!("Failed to parse address");
				}])
				.build();

			let sub = web3.eth_subscribe().subscribe_logs(filter).await;

			if let Ok(mut sub) = sub {
				while let Some(log) = sub.next().await {
					match log {
						Ok(log) =>
							if let Ok(log_in_bytes) = serialize(&log) {
								let hash = Self::hash_keccak_256(&log_in_bytes);
								match self.sign_data_sender.lock().await.try_send((1, hash)) {
									Ok(()) => {
										log::info!("Connector successfully send event to channel")
									},
									Err(_) => {
										log::info!("Connector failed to send event to channel")
									},
								}
							} else {
								log::info!("Failed to serialize log: {:?}", log);
							},
						Err(e) => log::warn!("Error on log {:?}", e),
					}
				}
			} else {
				log::info!("Failed to subscribe to logs: {:?}", sub);
			}
		}
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(3);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				// Get swap data from db and send it to time-worker
				let tasks_in_byte = Self::get_swap_data_from_db();
				if !tasks_in_byte.is_empty() {
					for task in tasks_in_byte.iter() {
						let result = sign_data_sender_clone.lock().await.try_send((1, *task));
						match result {
							Ok(_) => warn!("sign_data_sender_clone ok"),
							Err(_) => warn!("sign_data_sender_clone err"),
						}
					}
				}

				// Get latest block event from Uniswap v2 and send it to time-worker
				Self::get_latest_block_event(self).await;
				thread::sleep(delay);
			}
		}
	}
}
