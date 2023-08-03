use crate::TaskExecutorError;
use crate::{BlockHeight, TaskExecutorParams, TW_LOG};
use anyhow::{Context, Result};
use codec::{Decode, Encode};
use dotenv::dotenv;
use futures::channel::{mpsc, oneshot};
use futures::SinkExt;
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
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::env;
use std::{
	collections::{HashMap, HashSet, VecDeque},
	fmt,
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use time_primitives::{
	abstraction::{Function, OCWSkdData, ScheduleStatus},
	ScheduleCycle, TaskId, TaskSchedule, TimeApi, TimeId, OCW_SKD_KEY, TIME_KEY_TYPE,
};
use time_worker::TssRequest;
use timechain_integration::query::{collect_data, CollectData};
use time_primitives::SignatureData;

#[derive(Clone)]
pub struct TaskExecutor<B, BE, R, A, BN> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	_account_id: PhantomData<A>,
	_block_number: PhantomData<BN>,
	sign_data_sender: mpsc::Sender<TssRequest>,
	kv: KeystorePtr,
	// all tasks that are scheduled
	// TODO need to get all completed task and remove them from it
	tasks: HashSet<u64>,
	error_count: HashMap<u64, u64>,
	rosetta_chain_config: BlockchainConfig,
	rosetta_client: Client,
	repetitive_tasks: HashMap<BlockHeight, Vec<(u64, u64, TaskSchedule<A>)>>,
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
		task_id: u64,
		cycle: u64,
	) -> Result<()> {
		let data = bincode::serialize(&data).context("Failed to serialize task")?;
		let (tx, rx) = oneshot::channel::<Option<SignatureData>>();
		self.sign_data_sender
			.send(TssRequest {
				request_id: (task_id, cycle),
				shard_id,
				data,
				tx,
			})
			.await?;

		
		let signature_data = rx.await?.ok_or(anyhow::anyhow!("Unable to compute signature"))?;
		//send signature_data to ocw
		println!("received signature from tss {:?}", signature_data);
		Ok(())
	}
	/// Fetches and executes contract call for a given schedule_id
	async fn task_executor(
		&mut self,
		schedule_id: &u64,
		schedule: &TaskSchedule<A>,
	) -> Result<CallResponse, TaskExecutorError> {
		match &schedule.function {
			// If the task function is an Ethereum contract
			// call, call it and send for signing
			Function::EVMViewWithoutAbi {
				address,
				function_signature,
				input,
			} => {
				log::info!("running schedule_id {:?}", schedule_id);
				match self.call_eth_contract(address, function_signature, input).await {
					Ok(data) => Ok(data),
					Err(e) => Err(TaskExecutorError::ExecutionError(e.to_string())),
				}
			},
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

	async fn send_data(
		&mut self,
		block_id: <B as Block>::Hash,
		target_block_number: BlockHeight,
		data: CallResponse,
		collection: String,
	) -> Result<(), Error> {
		// Add data into collection (user must have Collector role)
		// @collection: collection hashId
		// @cycle: time-chain block number
		// @block: target network block number
		// @task_id: task associated with data
		// @task_counter: for repeated task it's incremented on every run
		// @tss: TSS signature
		// @data: data to add into collection

		let data_value = match data.result {
			Value::Array(val) => val
				.iter()
				.filter_map(|x| x.as_str())
				.map(|x| x.to_string())
				.collect::<Vec<String>>(),
			v => vec![v.to_string()],
		};
		let variables = collect_data::Variables {
			collection,
			block: target_block_number as i64,
			//Todo add cycle here
			cycle: 0,
			// unused field
			task_id: 0,
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
				let client = reqwest::Client::new();
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

	async fn process_tasks_for_block(
		&mut self,
		block_id: <B as Block>::Hash,
		block_height: BlockHeight,
	) -> Result<()> {
		let Some(account) = self.account_id() else {
			anyhow::bail!("No account id found");
		};

		let shards = self.runtime.runtime_api().get_shards(block_id, account).unwrap();
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id).unwrap();
			for task_id in tasks {
				if self.tasks.contains(&task_id) {
					continue;
				}
				let task = self.runtime.runtime_api().get_task(block_id, task_id).unwrap().unwrap();

				// put the new task in repetitive task map
				let align_block_height = block_height + task.frequency;
				log::info!("Aligned height {:?} for schedule {:?}", align_block_height, task_id);
				self.tasks.insert(task_id);
				self.repetitive_tasks
					.entry(align_block_height)
					.or_insert(vec![])
					.push((task_id, shard_id, task));
			}
		}

		let total_items = self.repetitive_tasks.values().clone().flatten().collect::<Vec<_>>();
		log::info!("Available schedules {:?}", total_items.len());

		// iterate all block height
		for index in self.last_block_height..block_height {
			log::debug!("Iterating index {:?}", index);
			if let Some(tasks) = self.repetitive_tasks.remove(&index) {
				log::debug!("Task running on block {:?}", index);

				// execute all task for specific task
				for (task_id, shard_id, task) in tasks {
					match self.task_executor(&task_id, &task).await {
						Ok(data) => {
							//send for signing
							if let Err(e) = self
								.send_for_sign(data.clone(), shard_id, task_id, task.cycle)
								.await
							{
								log::error!("Error occurred while sending data for signing: {}", e);
							};

							let mut decremented_task = task.clone();
							decremented_task.cycle = decremented_task.cycle.saturating_sub(1);

							// put the task in map for next execution if cycle more than once
							if decremented_task.cycle > 0 {
								self.repetitive_tasks
									.entry(index + decremented_task.frequency)
									.or_insert(vec![])
									.push((task_id, shard_id, decremented_task));
							}
							self.error_count.remove(&task_id);
							match self
								.send_data(block_id, block_height, data, task.hash.to_owned())
								.await
							{
								Ok(()) => log::info!("Submit to TimeGraph successful"),
								Err(e) => log::warn!("Error on submit to TimeGraph {:?}", e),
							};
						},
						Err(e) => match e {
							TaskExecutorError::NoTaskFound => {
								log::error!("No task found for id {:?}", task_id);
								self.report_schedule_invalid(
									task_id, task.cycle, true, block_id, shard_id,
								);
							},
							TaskExecutorError::InvalidTaskFunction => {
								log::error!("Invalid task function provided");
								self.report_schedule_invalid(
									task_id, task.cycle, true, block_id, shard_id,
								);
							},
							TaskExecutorError::ExecutionError(error) => {
								log::error!(
									"Error occured while executing contract call {:?}: {}",
									task_id,
									error
								);

								if task.is_repetitive_task() {
									let is_terminated = self.report_schedule_invalid(
										task_id, task.cycle, false, block_id, shard_id,
									);

									// if not terminated keep add task with added frequency
									if !is_terminated {
										self.repetitive_tasks
											.entry(index + task.frequency)
											.or_insert(vec![])
											.push((task_id, shard_id, task));
									}
								} else {
									self.report_schedule_invalid(
										task_id, task.cycle, true, block_id, shard_id,
									);
								}
							},
							TaskExecutorError::InternalError(error) => {
								log::error!(
									"Internal error occured while processing task: {}",
									error
								);
								self.report_schedule_invalid(
									task_id, task.cycle, true, block_id, shard_id,
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

	/// Add schedule update task to offchain storage
	/// which will be use by offchain worker to send extrinsic
	fn update_schedule_ocw_storage(
		&mut self,
		_task_id: TaskId,
		_cycle: ScheduleCycle,
		_status: ScheduleStatus,
	) {
		// TODO
		/*let ocw_skd = OCWSkdData::new(task_id, );

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
		};*/
	}

	fn report_schedule_invalid(
		&mut self,
		task_id: TaskId,
		cycle: ScheduleCycle,
		terminate: bool,
		_blocknumber: <B as Block>::Hash,
		_shard_id: u64,
	) -> bool {
		let is_collector = false; //self.is_collector(blocknumber, shard_id).unwrap_or(false);
		if terminate {
			if is_collector {
				self.update_schedule_ocw_storage(task_id, cycle, ScheduleStatus::Invalid);
			}
			return true;
		}

		let error_count = self.error_count.entry(task_id).or_insert(0);
		*error_count += 1;

		if *error_count > 2 {
			if is_collector {
				self.update_schedule_ocw_storage(task_id, cycle, ScheduleStatus::Invalid);
			}
			self.error_count.remove(&task_id);
			return true;
		}
		false
	}

	pub async fn run(&mut self) {
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
					if let Err(e) = self.process_tasks_for_block(at, current_block).await {
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
}
