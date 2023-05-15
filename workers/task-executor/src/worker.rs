#![allow(clippy::type_complexity)]
use crate::{WorkerParams, TW_LOG};
use bincode::serialize;
use chrono::Utc;
use codec::Decode;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use rosetta_client::{
	create_client,
	types::{CallRequest, CallResponse},
	BlockchainConfig, Client,
};
use sc_client_api::Backend;
use serde_json::json;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_io::hashing::keccak_256;
use sp_runtime::{traits::Block, DispatchError};
use std::{collections::HashMap, env, error::Error, marker::PhantomData, sync::Arc};
use time_primitives::{
	abstraction::{Function, ScheduleStatus},
	TimeApi, TimeId,
};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time::sleep};
use worker_aurora::{establish_connection, models::Feeds, write_data_to_db};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, A, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
	account_id: PhantomData<A>,
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
			account_id: _,
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
			account_id: PhantomData,
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

	fn update_task_schedule_status(
		&self,
		block_id: <B as Block>::Hash,
		status: ScheduleStatus,
		schedule_task_id: u64,
	) -> Result<(), DispatchError> {
		match self
			.runtime
			.runtime_api()
			.update_schedule_by_key(block_id, status, schedule_task_id)
		{
			Ok(update) => update,
			Err(_) => Err(DispatchError::CannotLookup),
		}
	}

	async fn call_contract_and_send_for_sign(
		&self,
		block_id: <B as Block>::Hash,
		data: CallResponse,
		shard_id: u64,
		schedule_task_id: u64,
		map: &mut HashMap<u64, ()>,
	) -> Result<bool, Box<dyn Error>> {
		dotenv().ok();

		if let Ok(task_in_bytes) = serialize(&data.result) {
			let hash = Self::hash_keccak_256(&task_in_bytes);

			let at = self.backend.blockchain().last_finalized();
			match at {
				Ok(at) =>
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
								let result =
									self.sign_data_sender.lock().await.try_send((shard_id, hash));
								if result.is_ok() {
									log::info!("Connector successfully send event to channel");
									map.insert(schedule_task_id, ());

									match Self::update_task_schedule_status(
										self,
										block_id,
										ScheduleStatus::Completed,
										schedule_task_id,
									) {
										Ok(()) =>
											log::info!("updated schedule status to completed"),
										Err(e) => log::warn!(
											"getting error on updating schedule status {:?}",
											e
										),
									}
									return Ok(true);
								} else {
									log::info!("Connector failed to send event to channel");
									match Self::update_task_schedule_status(
										self,
										block_id,
										ScheduleStatus::Invalid,
										schedule_task_id,
									) {
										Ok(()) => log::info!("updated schedule status to Canceled"),
										Err(e) => log::warn!(
											"getting error on updating schedule status {:?}",
											e
										),
									}
								}
							} else {
								log::info!("shard not same");
							}
						} else {
							log::error!(target: TW_LOG, "task-executor no matching shard found");
						}
					} else {
						log::error!(target: TW_LOG, "Failed to construct account");
					},
				Err(e) => log::warn!("error at getting last finalized block {:?}", e),
			}
		} else {
			log::info!("Failed to serialize task: {:?}", data);
		}
		Ok(false)
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
					let shard_id = schedule_task.1.shard_id;
					if !map.contains_key(&schedule_task.0) {
						let metadata_result = self
							.runtime
							.runtime_api()
							.get_task_metadata_by_key(block_id, schedule_task.0);
						if let Ok(metadata_result) = metadata_result {
							match metadata_result {
								Ok(metadata) => {
									match metadata {
										Some(task) => {
											match &task.function {
												// If the task function is an Ethereum contract
												// call, call it and send for signing
												Function::EthereumContractWithoutAbi {
													address,
													function_signature,
													input: _,
													output: _,
												} => {
													let method = format!(
														"{address}-{function_signature}-call"
													);
													let request = CallRequest {
														network_identifier: config.network(),
														method,
														parameters: json!({}),
													};

													let data = client.call(&request).await?;

													match Self::call_contract_and_send_for_sign(
														self,
														block_id,
														data,
														shard_id,
														schedule_task.0,
														map,
													)
													.await
													{
														Ok(true) => {
															let conn_url = env::var("DATABASE_URL").map_err(|_| "Error the DATABASE_URL not set.")?;
															let pg_conn = establish_connection(
																Some(&conn_url),
															);

															let id:i64 = schedule_task.0.try_into().unwrap();
															let hash = schedule_task.1.hash.to_owned();
															let value = match serde_json::to_value(task.clone()){
																Ok(value) => value,
																Err(e) => {
																	log::warn!("Error serializing task: {:?}", e);
																	serde_json::Value::Null
																}
															};
															let task = value.to_string().as_bytes().to_vec();
															let validity = 123;
															let timestamp = Some(Utc::now().naive_utc());
															let cycle = Some(schedule_task.1.cycle.try_into().unwrap());

															let record = Feeds {
																id,
									 							hash,
																task,
																timestamp,
																validity,
																cycle,
															};
															match pg_conn {
																Ok(mut pg_conn) => write_data_to_db(&mut pg_conn, record),
																Err(e) => log::warn!("Error in connection {:?}",e),
															}
														},
														Ok(false) => {
															log::warn!("status not updated can't updated data into DB")
														},
														Err(_) => log::warn!("Error on call contract and send for sign"),
													}
												},
												_ => {
													log::warn!("error on matching task function")
												},
											};
										},
										None => {
											log::info!("task schedule id have no metadata, Removing task from Schedule list");
											match Self::update_task_schedule_status(
												self,
												block_id,
												ScheduleStatus::Invalid,
												schedule_task.0,
											) {
												Ok(()) => log::info!("The schedule status has been updated to Invalid"),
												Err(e) =>
													log::warn!("getting error on updating schedule status {:?}", e),
											}
											//to-do Remove task from schedule list
										},
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
					} else {
						log::info!(
							"The key didn't exist and was inserted key {:?}.",
							schedule_task.0
						);
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
