use crate::{TaskExecutorParams, TW_LOG};
use anyhow::{Context, Result};
use codec::{Decode, Encode};
use futures::channel::mpsc::Sender;
use rosetta_client::{
	create_client,
	types::{BlockRequest, CallRequest, CallResponse, PartialBlockIdentifier},
	BlockchainConfig, Client,
};
use sc_client_api::Backend;
use serde_json::json;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as _;
use sp_core::{hashing::keccak_256, offchain::STORAGE_PREFIX};
use sp_keystore::KeystorePtr;
use sp_runtime::offchain::OffchainStorage;
use sp_runtime::traits::Block;
use std::{
	collections::{BTreeMap, HashSet, VecDeque},
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use time_db::{feed::Model, fetch_event::Model as FEModel, DatabaseConnection};
use time_primitives::{
	abstraction::{Function, OCWSkdData, ScheduleStatus},
	KeyId, TaskSchedule, TimeApi, TimeId, OCW_SKD_KEY, TIME_KEY_TYPE,
};

use queue::Queue;
use std::collections::HashMap;

pub struct TaskExecutor<B, BE, R, A> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	_account_id: PhantomData<A>,
	sign_data_sender: Sender<(u64, u64, [u8; 32])>,
	kv: KeystorePtr,
	tasks: HashSet<u64>,
	task_map: HashMap<u64, Vec<(u64, TaskSchedule<A>)>>,
	db: DatabaseConnection,
	chain_config: BlockchainConfig,
	chain_client: Client,
}

impl<B, BE, R, A> TaskExecutor<B, BE, R, A>
where
	B: Block,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	A: codec::Codec + Clone,
	R::Api: TimeApi<B, A>,
{
	pub async fn new(params: TaskExecutorParams<B, A, R, BE>) -> Result<Self> {
		let TaskExecutorParams {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block,
			accountid: _,
			connector_url,
			connector_blockchain,
			connector_network,
		} = params;

		let (chain_config, chain_client) =
			create_client(connector_blockchain, connector_network, connector_url).await?;

		let db = time_db::connect().await?;

		Ok(Self {
			_block: PhantomData,
			backend,
			runtime,
			_account_id: PhantomData,
			sign_data_sender,
			kv,
			tasks: Default::default(),
			task_map: Default::default(),
			db,
			chain_config,
			chain_client,
		})
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

	async fn call_contract_and_send_for_sign(
		&mut self,
		block_id: <B as Block>::Hash,
		data: CallResponse,
		shard_id: u64,
		task_id: u64,
		id: u64,
	) -> Result<bool> {
		let bytes = bincode::serialize(&data.result).context("Failed to serialize task")?;
		let hash = keccak_256(&bytes);

		self.sign_data_sender.clone().try_send((shard_id, task_id, hash))?;
		self.tasks.insert(id);

		if self.is_collector(block_id, shard_id).unwrap_or(false) {
			self.update_schedule_ocw_storage(ScheduleStatus::Completed, id);
		}

		Ok(true)
	}

	async fn task_executor(
		&mut self,
		block_id: <B as Block>::Hash,
		id: &u64,
		schedule: &TaskSchedule<A>,
	) -> Result<()> {
		if !self.tasks.contains(id) {
			let metadata = self
				.runtime
				.runtime_api()
				.get_task_metadat_by_key(block_id, schedule.task_id.0)?
				.map_err(|err| anyhow::anyhow!("{:?}", err))?;

			let shard_id = schedule.shard_id;
			let Some(task) = metadata else {
					log::info!("task schedule id have no metadata, Removing task from Schedule list");

					if self.is_collector(block_id, shard_id).unwrap_or(false) {
						self.update_schedule_ocw_storage(ScheduleStatus::Invalid, *id);
					}

					return Ok(());
				};

			let task_schedule_clone = schedule.clone();
			match &task.function {
				// If the task function is an Ethereum contract
				// call, call it and send for signing
				Function::EthereumViewWithoutAbi {
					address,
					function_signature,
					input: _,
					output: _,
				} => {
					log::info!("running task_id {:?}", id);
					let method = format!("{address}-{function_signature}-call");
					let request = CallRequest {
						network_identifier: self.chain_config.network(),
						method,
						parameters: json!({}),
					};
					let data = self.chain_client.call(&request).await?;
					if !self
						.call_contract_and_send_for_sign(
							block_id,
							data.clone(),
							shard_id,
							task_schedule_clone.task_id.0,
							*id,
						)
						.await?
					{
						log::warn!("status not updated can't updated data into DB");
						return Ok(());
					}
					let id: i64 = (*id).try_into().unwrap();
					let hash = task.hash.to_owned();
					let value = match serde_json::to_value(task.clone()) {
						Ok(value) => value,
						Err(e) => {
							log::warn!("Error serializing task: {:?}", e);
							serde_json::Value::Null
						},
					};
					let validity = 123;
					let cycle = Some(task.cycle.try_into().unwrap());
					let task = value.to_string().as_bytes().to_vec();
					let record = Model {
						id: 1,
						task_id: id,
						hash,
						task,
						timestamp: None,
						validity,
						cycle,
					};

					match serde_json::to_string(&data) {
						Ok(response) => {
							let fetch_record = FEModel {
								id: 1,
								block_number: 1,
								cycle,
								value: response,
							};
							let _ = time_db::write_fetch_event(&mut self.db, fetch_record).await;
						},
						Err(e) => log::info!("getting error on serde data {e}"),
					};

					time_db::write_feed(&mut self.db, record).await?;
				},
				_ => {
					log::warn!("error on matching task function")
				},
			};
		}
		Ok(())
	}

	fn is_collector(&self, block_id: <B as Block>::Hash, shard_id: u64) -> Result<bool> {
		let Some(account) = self.account_id() else {
			return Ok(false);
		};

		let Some(shard) = self
							.runtime
							.runtime_api()
							.get_shards(block_id)
							.unwrap_or(vec![])
							.into_iter()
							.find(|(s, _)| *s == shard_id)
							.map(|(_, s)| s) else {
			anyhow::bail!("failed to find shard");
		};

		Ok(*shard.collector() == account)
	}

	async fn process_tasks_for_block(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let task_schedules = self
			.runtime
			.runtime_api()
			.get_task_schedule(block_id)?
			.map_err(|err| anyhow::anyhow!("{:?}", err))?;

		let mut tree_map = BTreeMap::new();
		let mut queue = Queue::new();

		for (id, schedule) in task_schedules {
			tree_map.insert(id, schedule);
		}

		let block_request = BlockRequest {
			network_identifier: self.chain_config.network(),
			block_identifier: PartialBlockIdentifier { index: None, hash: None },
		};
		let response = self.chain_client.block(&block_request).await?;

		for (id, schedule) in tree_map.iter() {
			if schedule.cycle == 1 {
				let _ = queue.queue((*id, schedule.clone()));
			} else {
				match response.clone().block {
					Some(block) => {
						self.task_map
							.entry(block.block_identifier.index)
							.or_insert(Vec::new())
							.push((*id, schedule.clone()));
					},
					None => log::info!("failed to get BlockResponse from rosetta"),
				}
			}
		}

		while let Some(single_task_schedule) = queue.dequeue() {
			if let Err(e) = self
				.task_executor(block_id, &single_task_schedule.0, &single_task_schedule.1)
				.await
			{
				log::error!("Error occured while executing task: {}", e);
			};
		}
		let key: u64;

		{
			let min_key = self.task_map.keys().min();
			key = match min_key {
				Some(key) => *key,
				None => 0,
			};
		}
		match response.block {
			Some(block) => {
				let block_number = if block.block_identifier.index >= key {
					block.block_identifier.index
				} else {
					key
				};
				if let Some(tasks) = self.task_map.remove(&block_number) {
					for recursive_task_schedule in tasks {
						let result = self
							.task_executor(
								block_id,
								&recursive_task_schedule.0,
								&recursive_task_schedule.1,
							)
							.await;

						match result {
							Ok(()) => {
								if recursive_task_schedule.1.cycle > 1
									&& recursive_task_schedule.1.status == ScheduleStatus::Recurring
								{
									//Update Extrinsic with cycle count-1;
									todo!();
								} else if recursive_task_schedule.1.cycle > 1 {
									//Update cycle count-1 and status = Recursive;
									todo!();
								} else {
									//Update status = Compelete
								}

								if recursive_task_schedule.1.cycle > 1 {
									// Updating HashMap key and value, because not going to retrive this task again from task schedule
									self.task_map
										.entry(block_number + recursive_task_schedule.1.frequency)
										.or_insert(Vec::new())
										.push((
											recursive_task_schedule.0,
											TaskSchedule {
												task_id: recursive_task_schedule.1.task_id,
												owner: recursive_task_schedule.1.owner,
												shard_id: recursive_task_schedule.1.shard_id,
												cycle: recursive_task_schedule.1.cycle - 1,
												frequency: recursive_task_schedule.1.frequency,
												validity: recursive_task_schedule.1.validity,
												hash: recursive_task_schedule.1.hash,
												start_execution_block: recursive_task_schedule
													.1
													.start_execution_block,
												status: ScheduleStatus::Recurring,
											},
										));
								}
							},
							Err(e) => log::warn!("error on result {:?}", e),
						}
					}
				}
			},
			None => log::info!("failed to get BlockResponse from rosetta"),
		}
		Ok(())
	}

	fn update_schedule_ocw_storage(&mut self, schedule_status: ScheduleStatus, key: KeyId) {
		let ocw_skd = OCWSkdData::new(schedule_status, key);

		if let Some(mut ocw_storage) = self.backend.offchain_storage() {
			let old_value = ocw_storage.get(STORAGE_PREFIX, OCW_SKD_KEY);

			let mut ocw_vec = match old_value.clone() {
				Some(mut data) => {
					//remove this unwrap
					let mut bytes: &[u8] = &mut data;
					let inner_data: VecDeque<OCWSkdData> = Decode::decode(&mut bytes).unwrap();
					inner_data
				},
				None => Default::default(),
			};

			ocw_vec.push_back(ocw_skd);
			let encoded_data = Encode::encode(&ocw_vec);
			let is_data_stored = ocw_storage.compare_and_set(
				STORAGE_PREFIX,
				OCW_SKD_KEY,
				old_value.as_deref(),
				&encoded_data,
			);
			log::info!("stored task data in ocw {:?}", is_data_stored);
		} else {
			log::error!("cant get offchain storage");
		};
	}

	pub async fn run(&mut self) {
		loop {
			match self.backend.blockchain().last_finalized() {
				Ok(at) => {
					if let Err(e) = self.process_tasks_for_block(at).await {
						log::error!("Failed to process tasks for block {:?}: {:?}", at, e);
					}
				},
				Err(e) => {
					log::error!("Blockchain is empty: {}", e);
				},
			};
			tokio::time::sleep(Duration::from_secs(10)).await;
		}
	}
}
