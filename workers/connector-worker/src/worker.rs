#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use ink::env::hash;
use log::warn;
use rosetta_client::{create_wallet, types::PartialBlockIdentifier};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, thread};
use time_worker::kv::TimeKeyvault;
use tokio::sync::Mutex;
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

		let client_url = "http://127.0.0.1:8081";
		let contract_address = "0x678ea0447843f69805146c521afcbcc07d6e28a2";

		let wallet = create_wallet(
			Some("ethereum".to_owned()),
			Some("dev".to_owned()),
			Some(client_url.to_owned()),
			None,
		)
		.await
		.unwrap();

		let block_data =
			wallet.block(PartialBlockIdentifier { index: None, hash: None }).await.unwrap();

		println!("==================");
		println!("iterating block data");
		println!("==================");
		if let Some(data) = block_data.block {
			for tx in data.transactions {
				if let Some(metadata) = tx.metadata {
					if let Some(receipts) = metadata["receipt"]["logs"].as_array() {
						for log in receipts {
							let address = log["address"].as_str().unwrap();
							if address == contract_address {
								if let Ok(log_in_bytes) = serialize(&log) {
									log::info!("Connector received event: {:?}", log);
									let hash = Self::hash_keccak_256(&log_in_bytes);
									match self.sign_data_sender.lock().await.try_send((1, hash)) {
										Ok(()) => {
											log::info!(
												"Connector successfully send event to channel"
											)
										},
										Err(_) => {
											log::info!("Connector failed to send event to channel")
										},
									}
								} else {
									log::info!("Failed to serialize log: {:?}", log);
								}
							}
						}
					}
				}
			}
		};
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
				sleep(delay).await;
			}
		}
	}
}
