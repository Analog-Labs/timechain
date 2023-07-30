use crate::TaskExecutorError;
use crate::{BlockHeight, TaskExecutorParams, TW_LOG};
use anyhow::{Context, Result};
use codec::{Decode, Encode};
use dotenv::dotenv;
use futures::channel::mpsc::Sender;
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use reqwest::header;
use rosetta_client::{
	create_client,
	types::{CallRequest, CallResponse},
	BlockchainConfig, Client,
};
use sc_client_api::Backend;
use serde_json::{json, Value};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as _;
use sp_blockchain::HeaderBackend;
use sp_core::{hashing::keccak_256, offchain::STORAGE_PREFIX};
use sp_keystore::KeystorePtr;
use sp_runtime::offchain::OffchainStorage;
use sp_runtime::traits::Block;
use std::env;
use std::{
	collections::{BTreeMap, HashMap, HashSet, VecDeque},
	fmt,
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use frame_support::sp_tracing::error;
use time_primitives::{
	abstraction::{Function, OCWSkdData, ScheduleStatus},
	sharding::Network,
	KeyId, TaskSchedule, TimeApi, TimeId, OCW_SKD_KEY, TIME_KEY_TYPE,
};
use timechain_integration::query::{collect_data, CollectData};
#[derive(Clone)]
pub struct TaskExecutor<B, BE, R, A, BN> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	_account_id: PhantomData<A>,
	_block_number: PhantomData<BN>,
	sign_data_sender: Sender<(u64, u64, u64, [u8; 32])>,
	kv: KeystorePtr,
	// all tasks that are scheduled
	// TODO need to get all completed task and remove them from it
	tasks: HashSet<u64>,
	error_count: HashMap<u64, u64>,
	rosetta_chain_config: BlockchainConfig,
	rosetta_client: Client,
	repetitive_tasks: HashMap<BlockHeight, Vec<(u64, u64, TaskSchedule<A, BN>)>>,
	last_block_height: BlockHeight,
}

#[derive(Debug)]
enum Error {
	ErrorOnSendDataToTimeGraph,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::ErrorOnSendDataToTimeGraph => write!(f, "Faild to send data to Timegraph"),
		}
	}
}

impl<B, BE, R, A, BN> TaskExecutor<B, BE, R, A, BN>
where
	B: Block,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	A: codec::Codec + Clone,
	BN: codec::Codec + Clone,
	R::Api: TimeApi<B, A, BN>,
{
	pub async fn new(params: TaskExecutorParams<B, A, BN, R, BE>) -> Result<Self> {
		let TaskExecutorParams {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block,
			account_id: _,
			_block_number,
			connector_url,
			connector_blockchain,
			connector_network,
		} = params;

		// create rosetta client and get chain configuration
		let (rosetta_chain_config, rosetta_client) =
			create_client(connector_blockchain, connector_network, connector_url).await?;

		Ok(Self {
			_block: PhantomData,
			backend,
			runtime,
			_account_id: PhantomData,
			_block_number,
			sign_data_sender,
			kv,
			tasks: Default::default(),
			error_count: Default::default(),
			rosetta_chain_config,
			rosetta_client,
			repetitive_tasks: Default::default(),
			last_block_height: 0,
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

	/// Encode call response and send data for tss signing process
	async fn send_for_sign(
		&mut self,
		data: CallResponse,
		shard_id: u64,
		schedule_id: u64,
		schedule_cycle: u64,
	) -> Result<()> {
		let serialized_data = format!("{}-{}-{}", data.result, schedule_id, schedule_cycle);
		let bytes = bincode::serialize(&serialized_data).context("Failed to serialize task")?;
		let hash = keccak_256(&bytes);

		self.sign_data_sender
			.clone()
			.try_send((shard_id, schedule_id, schedule_cycle, hash))?;

		Ok(())
	}
	/// Fetches and executes contract call for a given schedule_id
	async fn task_executor(
		&mut self,
		block_id: <B as Block>::Hash,
		schedule_id: &u64,
		schedule: &TaskSchedule<A, BN>,
	) -> Result<CallResponse, TaskExecutorError> {
		let metadata = self
			.runtime
			.runtime_api()
			.get_task_metadata_by_key(block_id, schedule.task_id.0)
			.map_err(|err| TaskExecutorError::InternalError(err.to_string()))?
			.map_err(|err| TaskExecutorError::InternalError(format!("{:?}", err)))?;

		let Some(task) = metadata else {
			log::info!("No task found for id {:?}", schedule.task_id.0);
			return Err(TaskExecutorError::NoTaskFound(schedule.task_id.0));
		};

		match &task.function {
			// If the task function is an Ethereum contract
			// call, call it and send for signing
			Function::EVMViewWithoutAbi {
				address,
				function_signature,
				input,
				output: _,
			} => {
				log::info!("running schedule_id {:?}", schedule_id);
				match self.call_eth_contract(address, function_signature, input).await {
					Ok(data) => Ok(data),
					Err(e) => Err(TaskExecutorError::ExecutionError(e.to_string())),
				}
			},
			_ => Err(TaskExecutorError::InvalidTaskFunction),
		}
	}

	pub(crate) async fn call_eth_contract(
		&self,
		address: &str,
		function: &str,
		input: &[String],
	) -> Result<CallResponse> {
		let method = format!("{address}-{function}-call");
		let request = CallRequest {
			network_identifier: self.rosetta_chain_config.network(),
			method,
			parameters: json!(input),
		};
		self.rosetta_client.call(&request).await
	}

	/// check if current node is collector
	fn is_collector(&self, block_id: <B as Block>::Hash, shard_id: u64) -> Result<bool> {
		let Some(account) = self.account_id() else {
			return Ok(false);
		};

		let available_shards = self.runtime.runtime_api().get_shards(block_id).unwrap_or(vec![]);
		if available_shards.is_empty() {
			anyhow::bail!("No shards available");
		}
		let Some(shard) = available_shards
							.into_iter()
							.find(|(s, _)| *s == shard_id)
							.map(|(_, s)| s) else {
			anyhow::bail!("failed to find shard");
		};

		Ok(*shard.collector() == account)
	}
	async fn send_data(
		&mut self,
		block_id: <B as Block>::Hash,
		target_block_number: BlockHeight,
		data: CallResponse,
		collection: String,
		task_id: u64,
	) -> Result<(), Error> {
		let value = match self.backend.blockchain().number(block_id) {
			Ok(Some(val)) => val.to_string(),
			Ok(None) => "0".to_string(),
			Err(e) => {
				log::warn!("Error occurred: {:?}", e);
				"0".to_string()
			},
		};
		let timechain_block_number = match value.parse::<i64>() {
			Ok(parsed_value) if parsed_value == 0 => {
				log::warn!("error on block number");
				return Err(Error::ErrorOnSendDataToTimeGraph); // Return from the function if cycle value is 0.
			},
			Ok(parsed_value) => parsed_value,
			Err(e) => {
				log::error!("Failed to parse value: {:?}", e);
				return Err(Error::ErrorOnSendDataToTimeGraph); // Return from the function if parsing fails.
			},
		};

		// Add data into collection (user must have Collector role)
		// @collection: collection hashId
		// @cycle: time-chain block number
		// @block: target network block number
		// @task_id: task associated with data
		// @task_counter: for repeated task it's incremented on every run
		// @tss: TSS signature
		// @data: data to add into collection

		log::info!("DATA to collect {:#?}",data.result);

		let data_value = match data.result {
			Value::Array(val) => val
				.iter()
				.map(|x| x.to_string())
				.collect::<Vec<_>>(),
			v => {
				log::warn!("expected array of values as a rosetta result");
				vec![v.to_string()]
			},
		};

		let variables = collect_data::Variables {
			collection,
			block: target_block_number as i64,
			cycle: timechain_block_number,
			task_id: task_id as i64,
			data: data_value,
		};
		dotenv().ok();
		let Ok(url) = env::var("TIMEGRAPH_GRAPHQL_URL") else {
			log::warn!("Unable to get timegraph graphql url, Setting up default local url");
			return Err(Error::ErrorOnSendDataToTimeGraph)
			};
		match env::var("SSK") {
			Ok(ssk) => {
				// Build the GraphQL request
				let request = CollectData::build_query(variables);
				// Execute the GraphQL request
				let client = reqwest::Client::builder()
					.danger_accept_invalid_certs(true)
					.build().map_err(|e| {
						log::error!("timegraph http client failed to build: {e}");
						Error::ErrorOnSendDataToTimeGraph
					})?; // TODO: do we really need to handle this?

				let response =
					client.post(url).json(&request).header(header::AUTHORIZATION, ssk).send().await;

				match response {
					Ok(response) => {
						let json_response =
							response.json::<GraphQLResponse<collect_data::ResponseData>>().await;

						match json_response {
							Ok(json) => {
								if let Some(data) = json.data {
									log::info!(
										"timegraph migrate collect status: {:?}",
										data.collect.status
									);
								} else {
									log::info!(
										"timegraph migrate collect status fail: No response : {:?}",
										json.errors
									);
									return Err(Error::ErrorOnSendDataToTimeGraph);
								}
							},
							Err(e) => {
								log::info!("Failed to parse response: {:?}", e);
								return Err(Error::ErrorOnSendDataToTimeGraph);
							},
						};
					},
					Err(e) => {
						log::info!("error in post request to timegraph: {:?}", e);
						return Err(Error::ErrorOnSendDataToTimeGraph);
					},
				}
			},
			Err(e) => {
				log::info!("Unable to get timegraph sskey {:?}", e);
				return Err(Error::ErrorOnSendDataToTimeGraph);
			},
		};
		Ok(())
	}
	// entry point for task execution, triggered by each finalized block in the Timechain
	async fn process_tasks_for_block(
		&mut self,
		block_id: <B as Block>::Hash,
		target_block_number: BlockHeight,
	) -> Result<()> {
		let Some(account) = self.account_id() else {
			anyhow::bail!("No account id found");
		};

		let all_schedules = self
			.runtime
			.runtime_api()
			.get_one_time_task_schedule(block_id)?
			.map_err(|err| anyhow::anyhow!("{:?}", err))?;

		//filter schedules for this node's shard
		let task_schedules = all_schedules
			.into_iter()
			.filter_map(|(schedule_id, schedule)| {
				if let Ok(Ok(shard_id)) =
					self.runtime.runtime_api().get_task_shard(block_id, schedule_id)
				{
					let shard_members = self
						.runtime
						.runtime_api()
						.get_shard_members(block_id, shard_id)
						.unwrap_or(Some(vec![]))
						.unwrap_or(vec![]);

					if shard_members.contains(&account) {
						Some((schedule_id, shard_id, schedule))
					} else {
						None
					}
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		log::info!("single task schedule {:?}", task_schedules.len());

		let mut tree_map = BTreeMap::new();
		for (schedule_id, shard_id, schedule) in task_schedules {
			// if task is already executed then skip
			if self.tasks.contains(&schedule_id) {
				continue;
			}
			tree_map.insert(schedule_id, (shard_id, schedule));
			self.tasks.insert(schedule_id);
		}

		for (schedule_id, (shard_id, schedule)) in tree_map.iter() {
			//check if current shard is active
			if !self.is_current_shard_online(block_id, shard_id, schedule.network)? {
				//shard offline cant do any processing.
				self.repetitive_tasks.clear();
				self.tasks.clear();
				anyhow::bail!("Shard is offline id: {:?}", &shard_id);
			}
			match self.task_executor(block_id, schedule_id, schedule).await {
				Ok(data) => {
					if let Err(e) = self
						.send_for_sign(data.clone(), *shard_id, *schedule_id, schedule.cycle)
						.await
					{
						log::error!("Error occured while sending data for signing: {}", e);
					};
					match self
						.send_data(
							block_id,
							target_block_number,
							data,
							schedule.hash.to_owned(),
							schedule.task_id.0,
						)
						.await
					{
						Ok(()) => log::info!("Submit to TimeGraph successful"),
						Err(e) => log::warn!("Error on submit to TimeGraph {:?}", e),
					};
				},
				Err(e) => {
					//process error
					log::error!(
						"Error occured while executing one time schedule {:?}: {}",
						schedule_id,
						e
					);
					self.report_schedule_invalid(*schedule_id, true, block_id, *shard_id);
				},
			}
		}

		Ok(())
	}

	async fn process_repetitive_tasks_for_block(
		&mut self,
		block_id: <B as Block>::Hash,
		block_height: BlockHeight,
	) -> Result<()> {
		let Some(account) = self.account_id() else {
			anyhow::bail!("No account id found");
		};

		// get all initialized repetitive tasks
		let all_schedules = self
			.runtime
			.runtime_api()
			.get_repetitive_task_schedule(block_id)?
			.map_err(|err| anyhow::anyhow!("{:?}", err))?;

		// filter schedules for this node's shard
		let task_schedules = all_schedules
			.into_iter()
			.filter_map(|(schedule_id, schedule)| {
				if let Ok(Ok(shard_id)) =
					self.runtime.runtime_api().get_task_shard(block_id, schedule_id)
				{
					let shard_members = self
						.runtime
						.runtime_api()
						.get_shard_members(block_id, shard_id)
						.unwrap_or(Some(vec![]))
						.unwrap_or(vec![]);

					if shard_members.contains(&account) {
						Some((schedule_id, shard_id, schedule))
					} else {
						None
					}
				} else {
					None
				}
			})
			.collect::<Vec<_>>();

		for (schedule_id, shard_id, schedule) in task_schedules {
			// if task is already executed then skip
			if self.tasks.contains(&schedule_id) {
				continue;
			}

			// put the new task in repetitive task map
			let align_block_height = (block_height / schedule.frequency + 1) * schedule.frequency;
			self.tasks.insert(schedule_id);
			self.repetitive_tasks.entry(align_block_height).or_insert(vec![]).push((
				schedule_id,
				shard_id,
				schedule,
			));
		}

		let total_items = self.repetitive_tasks.values().clone().flatten().collect::<Vec<_>>();
		log::info!("Repetitive task schedule {:?}", total_items.len());

		// iterate all block height
		for index in self.last_block_height..=block_height {
			if let Some(tasks) = self.repetitive_tasks.remove(&index) {
				//check if current shard is active
				if let Some((_, shard_id, schedule)) = tasks.first() {
					if !self.is_current_shard_online(block_id, shard_id, schedule.network)? {
						//shard offline cant do any processing.
						self.repetitive_tasks.clear();
						self.tasks.clear();
						anyhow::bail!("Shard is offline id: {:?}", shard_id);
					}
				}

				log::debug!("Recurring task running on block {:?}", index);

				// execute all task for specific task
				for (schedule_id, shard_id, schedule) in tasks {
					match self.task_executor(block_id, &schedule_id, &schedule).await {
						Ok(data) => {
							//send for signing
							if let Err(e) = self
								.send_for_sign(data.clone(), shard_id, schedule_id, schedule.cycle)
								.await
							{
								log::error!("Error occurred while sending data for signing: {}", e);
							};

							let mut decremented_schedule = schedule.clone();
							decremented_schedule.cycle =
								decremented_schedule.cycle.saturating_sub(1);

							// put the task in map for next execution if cycle more than once
							if decremented_schedule.cycle > 0 {
								self.repetitive_tasks
									.entry(index + decremented_schedule.frequency)
									.or_insert(vec![])
									.push((schedule_id, shard_id, decremented_schedule));
							}
							self.error_count.remove(&schedule_id);
							match self
								.send_data(
									block_id,
									block_height,
									data,
									schedule.hash.to_owned(),
									schedule.task_id.0,
								)
								.await
							{
								Ok(()) => log::info!("Submit to TimeGraph successful"),
								Err(e) => log::warn!("Error on submit to TimeGraph {:?}", e),
							};
						},
						Err(e) => match e {
							TaskExecutorError::NoTaskFound(task) => {
								log::error!("No repetitive task found for id {:?}", task);
								self.report_schedule_invalid(schedule_id, true, block_id, shard_id);
							},
							TaskExecutorError::InvalidTaskFunction => {
								log::error!("Invalid task function provided");
								self.report_schedule_invalid(schedule_id, true, block_id, shard_id);
							},
							TaskExecutorError::ExecutionError(error) => {
								log::error!(
								"Error occured while executing repetitive contract call {:?}: {}",
								schedule_id,
								error
								);

								let is_terminated = self.report_schedule_invalid(
									schedule_id,
									false,
									block_id,
									shard_id,
								);

								// if not terminated keep add task with added frequency
								if !is_terminated {
									self.repetitive_tasks
										.entry(index + schedule.frequency)
										.or_insert(vec![])
										.push((schedule_id, shard_id, schedule));
								}
							},
							TaskExecutorError::InternalError(error) => {
								log::error!(
									"Internal error occured while processing task: {}",
									error
								);
							},
						},
					}
				}
			}
			self.last_block_height = index + 1;
		}

		Ok(())
	}

	fn is_current_shard_online(
		&self,
		block_id: <B as Block>::Hash,
		shard_id: &u64,
		network: Network,
	) -> Result<bool> {
		let active_shard = self.runtime.runtime_api().get_active_shards(block_id, network)?;
		let active_shard_id = active_shard.into_iter().map(|(id, _)| id).collect::<HashSet<_>>();
		log::debug!("active_shards {:?}", active_shard_id);
		Ok(active_shard_id.contains(shard_id))
	}

	/// Add schedule update task to offchain storage
	/// which will be use by offchain worker to send extrinsic
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

	fn report_schedule_invalid(
		&mut self,
		schedule_id: u64,
		terminate: bool,
		blocknumber: <B as Block>::Hash,
		shard_id: u64,
	) -> bool {
		let is_collector = self.is_collector(blocknumber, shard_id).unwrap_or(false);
		if terminate {
			if is_collector {
				self.update_schedule_ocw_storage(ScheduleStatus::Invalid, schedule_id);
			}
			return true;
		}

		let error_count = self.error_count.entry(schedule_id).or_insert(0);
		*error_count += 1;

		if *error_count > 2 {
			if is_collector {
				self.update_schedule_ocw_storage(ScheduleStatus::Invalid, schedule_id);
			}
			self.error_count.remove(&schedule_id);
			return true;
		}
		false
	}

	pub async fn run(&mut self) {
		loop {
			let Ok(status) = self.rosetta_client.network_status(self.rosetta_chain_config.network()).await else {
				log::warn!("Error occurred getting rosetta client status to get target block number");
				continue;
			};
			let target_block_number = status.current_block_identifier.index;
			match self.backend.blockchain().last_finalized() {
				Ok(at) => {
					if let Err(e) = self.process_tasks_for_block(at, target_block_number).await {
						log::error!("Failed to process tasks for block {:?}: {:?}", at, e);
					}
				},
				Err(e) => {
					log::error!("Blockchain is empty: {}", e);
				},
			}
			tokio::time::sleep(Duration::from_millis(1000)).await;
		}
	}

	pub async fn run_repetitive_task(&mut self) {
		loop {
			// get the external blockchain's block number
			let Ok(status) = self.rosetta_client.network_status(self.rosetta_chain_config.network()).await else {
				log::warn!("Error occurred getting rosetta client status to get target block number");
				continue;
			};
			let current_block = status.current_block_identifier.index;
			// update last block height if never set before
			if self.last_block_height == 0 {
				self.last_block_height = current_block;
			}
			// get the last finalized block number
			match self.backend.blockchain().last_finalized() {
				Ok(at) => {
					if let Err(e) = self.process_repetitive_tasks_for_block(at, current_block).await
					{
						log::error!(
							"Failed to process repetitive tasks for block {:?}: {:?}",
							at,
							e
						);
					}
				},
				Err(e) => {
					log::error!("Blockchain is empty: {}", e);
				},
			}
			tokio::time::sleep(Duration::from_millis(1000)).await;
		}
	}
}
