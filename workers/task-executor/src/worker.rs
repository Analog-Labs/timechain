use crate::{task_schedule::TaskScheduler, TaskExecutorParams, TW_LOG};
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
use sp_runtime::traits::Block;
use sp_runtime::{offchain::OffchainStorage, AccountId32};
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

#[derive(Clone)]
pub struct TaskExecutor<B, BE, R, A>
where
	BE: Clone,
	R: Clone,
{
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
	BE: Backend<B> + Clone + 'static,
	R: ProvideRuntimeApi<B> + std::marker::Sync + std::marker::Send + Clone + 'static,
	A: codec::Codec + Clone + std::marker::Send + std::marker::Sync + 'static,
	R::Api: TimeApi<B, A>,
{
	pub async fn new(params: Box<TaskExecutorParams<B, A, R, BE>>) -> Result<Self> {
	
	// let params = match Box::try_unwrap(params) {
    //     Ok(params) => {params}
    //     Err(_) => panic!("Failed to unwrap Arc"),
    // };
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
		} = *params;

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
		_block_id: <B as Block>::Hash,
		data: CallResponse,
		shard_id: u64,
		mut tasks: HashSet<u64>,
		sign_data_sender: Sender<(u64, u64, [u8; 32])>,
		task_id: u64,
		id: u64,
	) -> Result<bool> {
		let bytes = bincode::serialize(&data.result).context("Failed to serialize task")?;
		let hash = keccak_256(&bytes);

		sign_data_sender.clone().try_send((shard_id, task_id, hash))?;
		tasks.insert(id);

		Ok(true)
	}

	async fn task_executor(
		block_id: <B as Block>::Hash,
		backend: Arc<BE>,
		id: &u64,
		schedule: &TaskSchedule<A>,
		runtime: Arc<R>,
		tasks: HashSet<u64>,
		chain_config: BlockchainConfig,
		chain_client: Client,
		account: AccountId32,
		sign_data_sender: Sender<(u64, u64, [u8; 32])>,
		mut db: DatabaseConnection,
	) -> Result<()> {
		if !tasks.contains(id) {
			let metadata = runtime
				.runtime_api()
				.get_task_metadat_by_key(block_id, schedule.task_id.0)?
				.map_err(|err| anyhow::anyhow!("{:?}", err))?;

			let shard_id = schedule.shard_id;
			let Some(task) = metadata else {
					log::info!("task schedule id have no metadata, Removing task from Schedule list");
					if Self::is_collector(block_id, shard_id, runtime, account).unwrap_or(false) {
						Self::update_schedule_ocw_storage(ScheduleStatus::Invalid, *id, backend);
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
						network_identifier: chain_config.network(),
						method,
						parameters: json!({}),
					};
					let data = chain_client.call(&request).await?;
					if !Self::call_contract_and_send_for_sign(
						block_id,
						data.clone(),
						shard_id,
						tasks,
						sign_data_sender,
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
							let _ = time_db::write_fetch_event(&mut db, fetch_record).await;
						},
						Err(e) => log::info!("getting error on serde data {e}"),
					};
					time_db::write_feed(&mut db, record).await?;
					if schedule.cycle == 1
						&& Self::is_collector(block_id, shard_id, runtime, account).unwrap_or(false)
					{
						Self::update_schedule_ocw_storage(
							ScheduleStatus::Completed,
							id.try_into().unwrap(),
							backend,
						);
					}
				},
				_ => {
					log::warn!("error on matching task function")
				},
			};
		}
		Ok(())
	}

	fn is_collector(
		block_id: <B as Block>::Hash,
		shard_id: u64,
		runtime: Arc<R>,
		account: AccountId32,
	) -> Result<bool> {
		// let Some(account) = self.account_id() else {
		// 	return Ok(false);
		// };

		let Some(shard) = runtime
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

	async fn process_repetitive_task(
		block_id: <B as Block>::Hash,
		backend: Arc<BE>,
		recursive_task_schedule: (u64, TaskSchedule<A>),
		block_number: u64,
		account: AccountId32,
		runtime: Arc<R>,
		tasks: HashSet<u64>,
		mut task_map: HashMap<u64, Vec<(u64, TaskSchedule<A>)>>,
		chain_config: BlockchainConfig,
		chain_client: Client,
		sign_data_sender: Sender<(u64, u64, [u8; 32])>,
		db: DatabaseConnection,
	) -> Result<()> {
		let result = Self::task_executor(
			block_id,
			backend.clone(),
			&recursive_task_schedule.0,
			&recursive_task_schedule.1,
			runtime.clone(),
			tasks,
			chain_config,
			chain_client,
			account.clone(),
			sign_data_sender,
			db,
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
					//Update Extrinsic with cycle count-1 and status Recurring
					if Self::is_collector(
						block_id,
						recursive_task_schedule.1.shard_id,
						runtime,
						account,
					)
					.unwrap_or(false)
					{
						Self::update_schedule_ocw_storage(
							ScheduleStatus::Recurring,
							recursive_task_schedule.0,
							backend.clone(),
						);
					}
				} else {
					//Update status = Compelete
					if recursive_task_schedule.1.status != ScheduleStatus::Completed
						&& Self::is_collector(
							block_id,
							recursive_task_schedule.1.shard_id,
							runtime,
							account,
						)
						.unwrap_or(false)
					{
						Self::update_schedule_ocw_storage(
							ScheduleStatus::Completed,
							recursive_task_schedule.0,
							backend,
						);
					}
				}

				if recursive_task_schedule.1.cycle > 1 {
					// Updating HashMap key and value, because not going to retrive this task again from task schedule
					task_map
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
		Ok(())
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
			if let Err(e) = Self::task_executor(
				block_id,
				self.backend.clone(),
				&single_task_schedule.0,
				&single_task_schedule.1,
				self.runtime.clone(),
				self.tasks.clone(),
				self.chain_config.clone(),
				self.chain_client.clone(),
				self.account_id().unwrap().clone(),
				self.sign_data_sender.clone(),
				self.db.clone(),
			)
			.await
			{
				log::error!("Error occured while executing task: {}", e);
			};
		}

		match response.clone().block {
			Some(block) => {
				if let Some(tasks) = self.task_map.remove(&block.block_identifier.index) {
					for recursive_task_schedule in &tasks {
						let backend = self.backend.clone();
						let account = self.account_id().unwrap();
						let runtime = self.runtime.clone();
						let task_s = self.tasks.clone();
						let task_map = self.task_map.clone();
						let sign_data_sender = self.sign_data_sender.clone();
						let db = self.db.clone();
						let chain_config = self.chain_config.clone();
						let chain_client = self.chain_client.clone();

						let task_scheduler = Arc::new(
							TaskScheduler::new(chain_config.clone(), chain_client.clone()).await,
						);
						let cloned_task_scheduler = Arc::clone(&task_scheduler);

						let recursive_task_schedule_clone = recursive_task_schedule.clone();
						cloned_task_scheduler.register_callback(
							block.block_identifier.index + recursive_task_schedule.1.frequency,
							move |_client: &Client| {
								let _ = Self::process_repetitive_task(
									block_id,
									backend.clone(),
									recursive_task_schedule_clone.clone(),
									block.block_identifier.index,
									account.clone(),
									runtime.clone(),
									task_s.clone(),
									task_map.clone(),
									chain_config.clone(),
									chain_client.clone(),
									sign_data_sender.clone(),
									db.clone(),
								);
							},
						);
					}
				}
			},
			None => log::info!("failed to get BlockResponse from rosetta"),
		}

		Ok(())
	}

	fn update_schedule_ocw_storage(schedule_status: ScheduleStatus, key: KeyId, backend: Arc<BE>) {
		let ocw_skd = OCWSkdData::new(schedule_status, key);

		if let Some(mut ocw_storage) = backend.offchain_storage() {
			let old_value = ocw_storage.get(STORAGE_PREFIX, OCW_SKD_KEY);

			let mut ocw_vec: VecDeque<OCWSkdData> = match old_value.clone() {
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
