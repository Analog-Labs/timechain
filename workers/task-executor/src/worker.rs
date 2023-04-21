#![allow(clippy::type_complexity)]
use crate::{WorkerParams, TW_LOG};
use bincode::serialize;
use codec::Decode;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use rosetta_client::{create_client, types::CallRequest, BlockchainConfig, Client};
use sc_client_api::Backend;
use serde_json::json;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_io::hashing::keccak_256;
use sp_runtime::traits::Block;
use std::{collections::HashMap, error::Error, marker::PhantomData, sync::Arc};
use time_primitives::{abstraction::Function, TimeApi, TimeId};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time::sleep};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, A, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
	accountid: PhantomData<A>,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
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
			connector_url,
			connector_blockchain,
			connector_network,
		} = worker_params;

		TaskExecutor {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block: PhantomData,
			accountid: PhantomData,
			connector_url,
			connector_blockchain,
			connector_network,
		}
	}

	fn account_id(&self) -> Option<TimeId> {
		let keys = self.kv.public_keys();
		if keys.is_empty() {
			log::warn!(target: TW_LOG, "No time key found, please inject one.");
			None
		} else {
			let id = &keys[0];
			TimeId::decode(&mut id.as_ref()).ok()
		}
	}

	pub fn hash_keccak_256(input: &[u8]) -> [u8; 32] {
		keccak_256(input)
	}

	async fn call_contract_and_send_for_sign(
		&self,
		config: &BlockchainConfig,
		client: &Client,
		address: String,
		function_signature: String,
		shard_id: u64,
	) -> Result<(), Box<dyn Error>> {
		dotenv().ok();

		let method = format!("{address}-{function_signature}-call");
		let request = CallRequest {
			network_identifier: config.network(),
			method,
			parameters: json!({}),
		};

		let data = client.call(&request).await?;
		if let Ok(task_in_bytes) = serialize(&data.result) {
			let hash = Self::hash_keccak_256(&task_in_bytes);

			let at = self.backend.blockchain().last_finalized().unwrap();

			if let Some(my_key) = self.account_id() {
				let current_shard = self
					.runtime
					.runtime_api()
					.get_shards(at)
					.unwrap_or(vec![])
					.into_iter()
					.find(|(s, _)| *s == shard_id);

				if let Some(shard) = current_shard {
					if shard.1.collector() == &my_key {
						match self.sign_data_sender.lock().await.try_send((1, hash)) {
							Ok(()) => {
								log::info!("Connector successfully send event to channel")
							},
							Err(_) => {
								log::info!("Connector failed to send event to channel")
							},
						}
					} else {
						log::info!("shard not same");
					}
				} else {
					log::error!(target: TW_LOG, "task-executor no matching shard found");
				}
			} else {
				log::error!(target: TW_LOG, "Failed to construct account");
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
		config: &BlockchainConfig,
		client: &Client,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Get the task schedule for the current block
		let tasks_schedule = self.runtime.runtime_api().get_task_schedule(block_id)?;
		match tasks_schedule {
			Ok(task_schedule) => {
				for schedule_task in task_schedule.iter() {
					let task_id = schedule_task.0;
					let shard_id = schedule_task.1.shard_id;

					match map.get(&task_id) {
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

							if let Ok(metadata_result) = metadata_result {
								match metadata_result {
									Ok(metadata) => {
										if metadata.is_some() {
											map.insert(task_id, ());
										}

										for task in metadata.iter() {
											match &task.function {
												// If the task function is an Ethereum contract
												// call, call it and send for signing
												Function::EthereumContractWithoutAbi {
													address,
													function_signature,
													input: _,
													output: _,
												} => {
													let _result =
														Self::call_contract_and_send_for_sign(
															self,
															config,
															client,
															address.to_string(),
															function_signature.to_string(),
															shard_id,
														)
														.await;
												},
												_ => {
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

		let connector_config = create_client(
			self.connector_blockchain.clone(),
			self.connector_network.clone(),
			self.connector_url.clone(),
		)
		.await
		.ok();

		loop {
			// Get the public keys from the Key-Value store to check key is set
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				// Get the last finalized block from the blockchain
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					// let at = BlockId::Hash(at);

					if let Some((config, client)) = &connector_config {
						match self.process_tasks_for_block(at, &mut map, config, client).await {
							Ok(_) => (),
							Err(e) => {
								log::error!("Failed to process tasks for block {:?}: {:?}", at, e);
							},
						}
					} else {
						log::error!(
						"XXXXXXX-Connector-worker not running since no client available-XXXXXXX"
					);
					}
				} else {
					log::error!("Blockchain is empty");
				}
				sleep(delay).await;
			}
		}
	}
}
