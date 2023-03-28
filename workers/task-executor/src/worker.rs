#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use ink::env::hash;
use rosetta_client::{create_client, types::CallRequest, BlockchainConfig, Client};
use sc_client_api::Backend;
use serde_json::json;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_io::hashing::keccak_256;
use sp_runtime::traits::Block;
use std::{collections::HashMap, error::Error, marker::PhantomData, sync::Arc};
use time_primitives::{abstraction::Function, TimeApi};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time};

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
		function_signature: String,
	) -> Result<(), Box<dyn Error>> {
		dotenv().ok();

		let (config, client) = if let Ok(client_config) = create_connector_client().await {
			(client_config.0, client_config.1)
		} else {
			return Err("Failed to create connector client".into());
		};
		let contract_address = match address.parse::<H160>() {
			Ok(parsed_address) => parsed_address,
			Err(_) => {
				// handle the error here, for example by printing a message and exiting the program
				log::warn!("Error parsing contract address");
				H160::default()
			},
		};

		let data = client.call(&request).await?;

		if let Ok(task_in_bytes) = serialize(&data.result) {
			println!("received data: {:?}", data.result);
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
			log::info!("Failed to serialize task: {:?}", data);
		}
		Ok(())
	}

	async fn process_tasks_for_block(
		&self,
		block_id: <B as Block>::Hash,
		map: &mut HashMap<u64, ()>,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Get the task schedule for the current block
		let tasks_schedule = self.runtime.runtime_api().get_task_schedule(block_id)?;
		match tasks_schedule {
			Ok(task_schedule) => {
				for schedule_task in task_schedule.iter() {
					let shard_id = schedule_task.1.shard_id;

					match map.insert(schedule_task.0, ()) {
						Some(old_value) =>
							log::info!("The key already existed with the value {:?}", old_value),
						None => {
							log::info!(
								"The key didn't exist and was inserted key {:?}.",
								schedule_task.0
							);
							let metadata_result = self
								.runtime
								.runtime_api()
								.get_task_metadat_by_key(block_id, schedule_task.1.task_id.0);
							match metadata_result {
								Ok(metadata) => {
									match metadata {
										Ok(Some(task)) => {
											match task.function {
												// If the task function is an Ethereum contract
												// call, call it and send for signing
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
										},
										Ok(None) => {
											log::info!("No task function found");
										},
										Err(_) => log::info!("No task metadata found"),
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
		let mut map: HashMap<u64, ()> = HashMap::new();
		loop {
			// Get the public keys from the Key-Value store to check key is set
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				// Get the last finalized block from the blockchain
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					// let at = BlockId::Hash(at);
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

async fn create_connector_client() -> Result<(BlockchainConfig, Client), Box<dyn Error>> {
	let connector_url = std::env::var("CONNECTOR_URL").expect("CONNECTOR_URL must be set");
	let connector_blockchain =
		std::env::var("CONNECTOR_BLOCKCHAIN").expect("CONNECTOR_BLOCKCHAIN must be set");
	let connector_network =
		std::env::var("CONNECTOR_NETWORK").expect("CONNECTOR_NETWORK must be set");

	let (config, client) =
		create_client(Some(connector_blockchain), Some(connector_network), Some(connector_url))
			.await?;

	Ok((config, client))
}
