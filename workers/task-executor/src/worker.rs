#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use ink::env::hash;
use sc_client_api::Backend;
use serde_json::from_str;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;

use sp_runtime::{generic::BlockId, traits::Block};
use std::{error::Error, marker::PhantomData, sync::Arc, thread};
use time_primitives::{abstraction::Function, TimeApi};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time};
use web3::{
	contract::{Contract, Options},
	types::{Address, H160},
};

// use worker_aurora::{self, establish_connection, get_on_chain_data};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
}

impl<B, R, BE> TaskExecutor<B, R, BE>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, R, BE>) -> Self {
		let WorkerParams {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block,
		} = worker_params;

		TaskExecutor {
			backend,
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

	async fn call_contract_function(
		&self,
		address: String,
		abi: String,
		method: String,
	) -> Result<(), Box<dyn Error>> {
		dotenv().ok();
		let infura_url = std::env::var("INFURA_URL").expect("INFURA_URL must be set");

		let websocket_result = web3::transports::WebSocket::new(&infura_url).await;
		let websocket = match websocket_result {
			Ok(websocket) => websocket,
			Err(_) => web3::transports::WebSocket::new(&infura_url)
				.await
				.expect("Failed to create default websocket"),
		};
		let web3 = web3::Web3::new(websocket);

		// Load the contract ABI and address
		let contract_abi = from_str(&abi).unwrap();
		let contract_address = Address::from(address.parse::<H160>().unwrap());

		// Create a new contract instance using the ABI and address
		let contract = Contract::new(web3.eth(), contract_address, contract_abi);

		// Call the "getGreeting" function on the contract instance
		let greeting: String =
			contract.query(method.as_str(), (), None, Options::default(), None).await?;

		// if let Ok(task_in_bytes) = serialize(&greeting) {
		// 	let hash = Self::hash_keccak_256(&task_in_bytes);
		// 	match self.sign_data_sender.lock().await.try_send((1, hash)) {
		// 		Ok(()) => {
		// 			log::info!("Connector successfully send event to channel")
		// 		},
		// 		Err(_) => {
		// 			log::info!("Connector failed to send event to channel")
		// 		},
		// 	}
		// } else {
		// 	log::info!("Failed to serialize task: {:?}", greeting);
		// }

		println!("The greeting is: {}", greeting);

		Ok(())
	}

	pub(crate) async fn run(&mut self) {
		let delay = time::Duration::from_secs(10);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					let at = BlockId::Hash(at);

					if let Ok(tasks_schedule) = self.runtime.runtime_api().get_task_schedule(&at) {
						match tasks_schedule {
							Ok(task_schedule) => {
								// task_schedule.iter().for_each(|schedule_task|{
								for schedule_task in task_schedule.iter() {
									let task_id = schedule_task.task_id.0;
									if let Ok(metadata_result) = self
										.runtime
										.runtime_api()
										.get_task_metadat_by_key(&at, task_id)
									{
										match metadata_result {
											Ok(metadata) => {
												// metadata.iter().for_each(|task| {
												for task in metadata.iter() {
													match &task.function {
														Function::EthereumContract {
															address,
															abi,
															function,
															input: _,
															output: _,
														} => {
															let _x = Self::call_contract_function(
																self,
																address.to_string(),
																abi.to_string(),
																function.to_string(),
															)
															.await;
														},
														Function::EthereumApi {
															function: _,
															input: _,
															output: _,
														} => {
															todo!()
														},
													};
												}
											},
											Err(e) => {
												log::info!(
													"No metadata found for block {:?}: {:?}",
													at,
													e
												);
											},
										}
									} else {
										log::error!(
											"Failed to get task metadata for block {:?}",
											at
										);
									}
								}
							},
							Err(e) => {
								log::info!("No metadata found for block {:?}: {:?}", at, e);
							},
						}
					} else {
						log::error!("Failed to get task schedule for block {:?}", at);
					}
				} else {
					log::error!("Blockchain is empty");
				}
				thread::sleep(delay);
			}
		}
	}
}
