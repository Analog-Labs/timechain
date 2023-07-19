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
	TimeApi, TimeId, TIME_KEY_TYPE,
};
use tokio::time::sleep;

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct PayableTaskExecutor<B: Block, A, BN, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	tx_data_sender: Sender<Vec<u8>>,
	gossip_data_sender: Sender<Vec<u8>>,
	kv: KeystorePtr,
	accountid: PhantomData<A>,
	_block_number: PhantomData<BN>,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
}

impl<B, A, BN, R, BE> PayableTaskExecutor<B, A, BN, R, BE>
where
	B: Block,
	A: codec::Codec,
	BN: codec::Codec,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A, BN>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, A, BN, R, BE>) -> Self {
		let WorkerParams {
			backend,
			runtime,
			tx_data_sender,
			gossip_data_sender,
			kv,
			_block,
			accountid,
			_block_number,
			connector_url,
			connector_blockchain,
			connector_network,
		} = worker_params;

		PayableTaskExecutor {
			backend,
			runtime,
			tx_data_sender,
			gossip_data_sender,
			kv,
			_block,
			accountid,
			_block_number,
			connector_url,
			connector_blockchain,
			connector_network,
		}
	}

	fn account_id(&self) -> Option<TimeId> {
		let keys = self.kv.sr25519_public_keys(TIME_KEY_TYPE);
		if keys.is_empty() {
			log::warn!(target: TW_LOG, "No time key found, please inject one.");
			None
		} else {
			let id = &keys[0];
			TimeId::decode(&mut id.as_ref()).ok()
		}
	}

	///TODO! Payable task executor was unprioritized
	/// This needs to update task schedule status,
	fn update_task_schedule_status(
		&self,
		_block_id: <B as Block>::Hash,
		_status: ScheduleStatus,
		_schedule_task_id: u64,
	) -> Result<(), DispatchError> {
		// Implementation is wrong. we need to update payable schedule not normal schedule.
		// Todo! make extrinsic for payable task update
		// use ocw to submit the extrinsic.
		Ok(())
	}

	/// checks if current node is collector
	fn check_if_collector(&self, shard_id: u64) -> bool {
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

	/// Creates and send transaction for given contract call
	async fn create_tx(
		&self,
		wallet: &Wallet,
		call_params: (&str, &str, &[String]),
		map: &mut HashMap<u64, ()>,
		schedule_id: u64,
		shard_id: u64,
	) {
		let address = call_params.0;
		let function_signature = call_params.1;
		let input = call_params.2;
		match wallet.eth_send_call(address, function_signature, input, 0).await {
			Ok(tx) => {
				map.insert(schedule_id, ());
				log::info!("Successfully executed contract call {:?}", tx);
				let eth_tx_validation = EthTxValidation {
					blockchain: self.connector_blockchain.clone(),
					network: self.connector_network.clone(),
					url: self.connector_url.clone(),
					tx_id: tx.hash,
					contract_address: address.into(),
					shard_id,
					schedule_id,
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

	/// Process tasks for given block
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
				for (schedule_id, schedule) in task_schedule.iter() {
					if let Ok(Ok(shard_id)) =
						self.runtime.runtime_api().get_task_shard(block_id, *schedule_id)
					{
						if !map.contains_key(&task_id) {
							//Get the metadata from scheduled payable task by key
							let metadata_result = self
								.runtime
								.runtime_api()
								.get_payable_task_metadata_by_key(block_id, schedule.task_id.0);
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
														if self.check_if_collector(shard_id) {
															log::info!(
																"Running task {:?}",
																schedule.task_id.0
															);
															self.create_tx(
																wallet,
																(
																	address,
																	function_signature,
																	input,
																),
																map,
																*schedule_id,
																shard_id,
															)
															.await;
														}
													},
													_ => {
														log::warn!(
															"error on matching task function"
														)
													},
												};
											},
											None => {
												log::info!("task schedule id have no metadata, Removing task from Schedule list");
												match Self::update_task_schedule_status(
													self,
													block_id,
													ScheduleStatus::Invalid,
													*schedule_id,
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
							log::info!("Payable task already executed, Key {:?}.", schedule_id);
						}
					} else {
						log::warn!("error getting shard_id assigned to task_id")
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
			let keys = self.kv.sr25519_public_keys(TIME_KEY_TYPE);
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
