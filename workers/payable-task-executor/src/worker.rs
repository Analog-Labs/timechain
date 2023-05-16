#![allow(clippy::type_complexity)]
use crate::{WorkerParams, TW_LOG};
use codec::Decode;
use core::time;
use futures::channel::mpsc::Sender;
use rosetta_client::{create_client, create_wallet, BlockchainConfig, EthereumExt};
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_runtime::{traits::Block, DispatchError};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use time_primitives::{
	abstraction::{Function, ScheduleStatus},
	TimeApi, TimeId,
};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time::sleep};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct PayableTaskExecutor<B: Block, A, R, BE> {
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

impl<B, A, R, BE> PayableTaskExecutor<B, A, R, BE>
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

		PayableTaskExecutor {
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

	fn update_task_schedule_status(
		&self,
		block_id: <B as Block>::Hash,
		status: ScheduleStatus,
		schdule_task_id: u64,
	) -> Result<(), DispatchError> {
		match self
			.runtime
			.runtime_api()
			.update_schedule_by_key(block_id, status, schdule_task_id)
		{
			Ok(update) => update,
			Err(_) => Err(DispatchError::CannotLookup),
		}
	}

	fn check_if_connector(&self, shard_id: u64) -> bool {
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
						return shard.1.collector() == &my_key;
					} else {
						log::warn!("Shards does not match");
					}
				},
			Err(e) => {
				log::warn!("error at getting last finalized block {:?}", e);
			},
		}
		false
	}

	async fn process_tasks_for_block(
		&self,
		block_id: <B as Block>::Hash,
		map: &mut HashMap<u64, ()>,
		config: &BlockchainConfig,
		url: Option<String>,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Get the task schedule for the current block
		let tasks_schedule = self.runtime.runtime_api().get_payable_task_schedule(block_id)?;
		match tasks_schedule {
			Ok(task_schedule) => {
				for schedule_task in task_schedule.iter() {
					let shard_id = schedule_task.1.shard_id;
					if !map.contains_key(&schedule_task.0) {
						let metadata_result = self
							.runtime
							.runtime_api()
							.get_payable_task_metadata_by_key(block_id, schedule_task.0);
						if let Ok(metadata_result) = metadata_result {
							match metadata_result {
								Ok(metadata) => {
									match metadata {
										Some(task) => {
											match &task.function {
												// If the task function is an Ethereum contract
												// call, call it and send for signing
												Function::EthereumTxWithoutAbi {
													address,
													function_signature,
													input,
													output: _,
												} =>
													if self.check_if_connector(shard_id) {
														let blockchain =
															config.network().blockchain.clone();

														let network =
															config.network().network.clone();

														match create_wallet(
															Some(blockchain),
															Some(network),
															url.clone(),
															None,
														)
														.await
														{
															Ok(wallet) => {
																match wallet
																	.eth_send_call(
																		address,
																		function_signature,
																		input,
																	)
																	.await
																{
																	Ok(tx) => {
																		log::info!("Successfully executed contract call {:?}", tx);
																	},
																	Err(e) => {
																		log::error!("Error occured while processing contract call {:?}", e);
																	},
																}
															},
															Err(e) => {
																log::error!(
																"Error occured while creating wallet {:?}" , e
															);
															},
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

					if let Some((config, _client)) = &connector_config {
						match self
							.process_tasks_for_block(
								at,
								&mut map,
								config,
								self.connector_url.clone(),
							)
							.await
						{
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
				log::info!("in payable task worker");
				sleep(delay).await;
			}
		}
	}
}
