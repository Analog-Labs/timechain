#![allow(clippy::type_complexity)]
use crate::{WorkerParams, TW_LOG};
use codec::{Decode, Encode};
use core::time;
use futures::channel::mpsc::Sender;
use rosetta_client::{create_wallet, EthereumExt, Wallet};
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_keystore::KeystorePtr;
use sp_runtime::{traits::Block, DispatchError};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use time_primitives::{
	abstraction::{EthTxValidation, Function, ScheduleStatus},
	TimeApi, TimeId, KEY_TYPE,
};
use tokio::time::sleep;

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct PayableTaskExecutor<B: Block, A, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Sender<(u64, [u8; 32])>,
	tx_data_sender: Sender<Vec<u8>>,
	gossip_data_sender: Sender<Vec<u8>>,
	kv: KeystorePtr,
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
			tx_data_sender,
			gossip_data_sender,
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
			tx_data_sender,
			gossip_data_sender,
			kv,
			_block: PhantomData,
			accountid: PhantomData,
			connector_url,
			connector_blockchain,
			connector_network,
		}
	}

	fn account_id(&self) -> Option<TimeId> {
		let keys = self.kv.sr25519_public_keys(KEY_TYPE);
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

	fn check_if_connector(&self, shard_id: u64) -> bool {
		let at = self.backend.blockchain().last_finalized();
		match at {
			Ok(at) => {
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
				}
			},
			Err(e) => {
				log::warn!("error at getting last finalized block {:?}", e);
			},
		}
		false
	}

	async fn create_wallet_and_tx(
		&self,
		wallet: &Wallet,
		address: &str,
		function_signature: &str,
		input: &[String],
		map: &mut HashMap<u64, ()>,
		schedule_task_id: u64,
	) {
		match wallet.eth_send_call(address, function_signature, input, 0).await {
			Ok(tx) => {
				map.insert(schedule_task_id, ());
				log::info!("Successfully executed contract call {:?}", tx);
				let eth_tx_validation = EthTxValidation {
					blockchain: self.connector_blockchain.clone(),
					network: self.connector_network.clone(),
					url: self.connector_url.clone(),
					tx_id: tx.hash,
					contract_address: address.into(),
				};

				let encoded_data = eth_tx_validation.encode();
				self.tx_data_sender.clone().try_send(encoded_data.clone()).unwrap();
				self.gossip_data_sender.clone().try_send(encoded_data).unwrap();
				log::info!("Sent data to network for signing");
			},
			Err(e) => {
				log::error!("Error occurred while processing contract call {:?}", e);
			},
		}
	}

	async fn process_tasks_for_block(
		&self,
		block_id: <B as Block>::Hash,
		map: &mut HashMap<u64, ()>,
		wallet: &Wallet,
	) -> Result<(), Box<dyn std::error::Error>> {
		// Get the payable task schedule for the current block
		let tasks_schedule = self.runtime.runtime_api().get_payable_task_schedule(block_id)?;
		match tasks_schedule {
			Ok(task_schedule) => {
				for schedule_task in task_schedule.iter() {
					let shard_id = schedule_task.1.shard_id;
					if !map.contains_key(&schedule_task.0) {
						//Get the metadata from scheduled payable task by key
						let metadata_result = self
							.runtime
							.runtime_api()
							.get_payable_task_metadata_by_key(block_id, schedule_task.1.task_id.0);
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
												} => {
													if self.check_if_connector(shard_id) {
														self.create_wallet_and_tx(
															wallet,
															address,
															function_signature,
															input,
															map,
															schedule_task.0,
														)
														.await;
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
						log::info!("Payable task already executed, Key {:?}.", schedule_task.0);
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

		let blockchain = self.connector_blockchain.clone();

		let network = self.connector_network.clone();

		let res_wallet = create_wallet(blockchain, network, self.connector_url.clone(), None).await;

		loop {
			let keys = self.kv.sr25519_public_keys(KEY_TYPE);
			if !keys.is_empty() {
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					match &res_wallet {
						Ok(wallet) => {
							match self.process_tasks_for_block(at, &mut map, wallet).await {
								Ok(_) => (),
								Err(e) => {
									log::error!(
										"Failed to process tasks for block {:?}: {:?}",
										at,
										e
									);
								},
							}
						},
						Err(e) => {
							log::error!("Error occurred while creating wallet: {:?}", e);
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
