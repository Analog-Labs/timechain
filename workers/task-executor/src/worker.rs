#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use sc_client_api::Backend;
use serde_json::from_str;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_io::hashing::keccak_256;
use sp_runtime::{generic::BlockId, traits::Block};
use std::{collections::HashMap, error::Error, marker::PhantomData, sync::Arc};
use time_primitives::{abstraction::Function, TimeApi};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time::sleep};
use web3::{
	contract::{Contract, Options},
	types::H160,
};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, A, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
	pub accountid: PhantomData<A>,
}

impl<B, A, R, BE> TaskExecutor<B, A, R, BE>
where
	B: Block,
	A: codec::Codec,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, A, R, BE>) -> Self {
		let WorkerParams {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block,
			accountid: _,
		} = worker_params;

		TaskExecutor {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block: PhantomData,
			accountid: PhantomData,
		}
	}

	pub fn hash_keccak_256(input: &[u8]) -> [u8; 32] {
		keccak_256(input)
	}

	async fn call_contract_and_send_for_sign(
		&self,
		address: String,
		abi: String,
		method: String,
		shard_id: u64,
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
		let contract_abi = match from_str(&abi) {
			Ok(abi) => abi,
			Err(e) => {
				log::warn!("Error parsing contract ABI:  {}", e);
				return Err("Failed to parse contract ABI".into());
			},
		};
		let contract_address = match address.parse::<H160>() {
			Ok(parsed_address) => parsed_address,
			Err(parse_error) => {
				// handle the error here, for example by printing a message and exiting the program
				log::warn!("Error parsing contract address: {}", parse_error);
				std::process::exit(1);
			},
		};

		// Create a new contract instance using the ABI and address
		let contract = Contract::new(web3.eth(), contract_address, contract_abi);

		let greeting: String =
			contract.query(method.as_str(), (), None, Options::default(), None).await?;
		if let Ok(task_in_bytes) = serialize(&greeting) {
			let hash = Self::hash_keccak_256(&task_in_bytes);
			match self.sign_data_sender.lock().await.try_send((shard_id, hash)) {
				Ok(()) => {
					log::info!("Connector successfully send event to -- channel")
				},
				Err(_) => {
					log::info!("Connector failed to send event to channel")
				},
			}
		} else {
			log::info!("Failed to serialize task: {:?}", greeting);
		}
		Ok(())
	}

	async fn process_tasks_for_block(
		&self,
		block_id: BlockId<B>,
		map: &mut HashMap<u64, String>,
	) -> Result<(), Box<dyn std::error::Error>> {
		// let tasks_schedule = self.runtime.runtime_api().get_task_schedule(&block_id)?;
		// Get the task schedule for the current block
		let tasks_schedule = match self.runtime.runtime_api().get_task_schedule(&block_id) {
			Ok(task_schedule) => task_schedule,
			Err(_e) => Ok({
				log::error!("Failed to get task schedule for block {:?}", block_id);
				Vec::new() // Return an empty vector as the default value
			}),
		};

		match tasks_schedule {
			Ok(task_schedule) => {
				for schedule_task in task_schedule.iter() {
					let task_id = schedule_task.task_id.0;
					let shard_id = schedule_task.shard_id;

					match map.insert(task_id, "hash".to_string()) {
						Some(old_value) =>
							log::info!("The key already existed with the value {:?}", old_value),
						None => {
							log::info!("The key didn't exist and was inserted key {:?}.", task_id);
							let metadata_result = self
								.runtime
								.runtime_api()
								.get_task_metadat_by_key(&block_id, task_id)?;

							match metadata_result {
								Ok(metadata) => {
									for task in metadata.iter() {
										match &task.function {
											// If the task function is an Ethereum contract call,
											// call it and send for signing
											Function::EthereumContract {
												address,
												abi,
												function,
												input: _,
												output: _,
											} => {
												let _result =
													Self::call_contract_and_send_for_sign(
														self,
														address.to_string(),
														abi.to_string(),
														function.to_string(),
														shard_id,
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
									log::warn!(
										"Failed to get task metadata for block {:?} {:?}",
										block_id,
										e
									);
								},
							}
						},
					}
				}
			},
			Err(e) => log::warn!("getting error on task schedule {:?}", e),
		}

		Ok(())
	}

	pub(crate) async fn run(&mut self) {
		// Set the delay for the loop
		let delay = time::Duration::from_secs(10);
		let mut map: HashMap<u64, String> = HashMap::new();
		loop {
			// Get the public keys from the Key-Value store to check key is set
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				// Get the last finalized block from the blockchain
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					let at = BlockId::Hash(at);
					match self.process_tasks_for_block(at, &mut map).await {
						Ok(_) => (),
						Err(e) => {
							log::error!("Failed to process tasks for block {:?}: {:?}", at, e);
						},
					}
				} else {
					log::error!("Blockchain is empty");
				}
				sleep(delay).await;
			}
		}
	}
}
