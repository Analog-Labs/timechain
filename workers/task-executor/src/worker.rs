use crate::TaskExecutorError;
use crate::{BlockHeight, TaskExecutorParams, TW_LOG};
use anyhow::{Context, Result};
use codec::{Decode, Encode};
use futures::channel::mpsc::Sender;
use rosetta_client::{create_wallet, EthereumExt, Wallet};
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as _;
use sp_core::offchain::STORAGE_PREFIX;
use sp_keystore::KeystorePtr;
use sp_runtime::offchain::OffchainStorage;
use sp_runtime::traits::Block;
use std::{
	collections::{HashMap, HashSet, VecDeque},
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use time_primitives::{
	abstraction::{Function, OCWSkdData, ScheduleStatus},
	sharding::Network,
	KeyId, TaskSchedule, TimeApi, TimeId, OCW_SKD_KEY, TIME_KEY_TYPE,
};

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
	rosetta_client: Wallet,
	repetitive_tasks: HashMap<BlockHeight, Vec<(u64, u64, TaskSchedule<A, BN>)>>,
	last_block_height: BlockHeight,
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
		let rosetta_client =
			create_wallet(connector_blockchain, connector_network, connector_url, None).await?;

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

	async fn execute_function(&self, function: &Function) -> Result<()> {
		// TODO: do something with results
		match function {
			Function::EVMViewWithoutAbi {
				address,
				function_signature,
				input,
				output: _,
			} => {
				let _ =
					self.rosetta_client.eth_view_call(address, function_signature, input).await?;
			},
			Function::EthereumTxWithoutAbi {
				address,
				function_signature,
				input,
				output: _,
			} => {
				let _ = self
					.rosetta_client
					.eth_send_call(address, function_signature, input, 0)
					.await?;
			},
			_ => anyhow::bail!("unsupported function"),
		}
		Ok(())
	}

	// entry point for task execution, triggered by each finalized block in the Timechain
	async fn process_tasks_for_block(
		&mut self,
		block_id: <B as Block>::Hash,
		block_height: u64,
	) -> Result<()> {
		let Some(account) = self.account_id() else {
			anyhow::bail!("No account id found");
		};

		let all_schedules = self
			.runtime
			.runtime_api()
			.get_task_schedule(block_id)?
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

		log::info!("task schedule {:?}", task_schedules.len());

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

		// iterate all block height
		for index in self.last_block_height..=block_height {
			let Some(tasks) = self.repetitive_tasks.remove(&index) else {
				continue;
			};

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
				let metadata = self
					.runtime
					.runtime_api()
					.get_task_metadata_by_key(block_id, schedule.task_id.0)
					.map_err(|err| TaskExecutorError::InternalError(err.to_string()))?
					.map_err(|err| TaskExecutorError::InternalError(format!("{:?}", err)))?;

				let Some(task) = metadata else {
                       log::info!("No task found for id {:?}", schedule.task_id.0);
					continue;
               };
				match self.execute_function(&task.function).await {
					Ok(data) => {
						let mut decremented_schedule = schedule.clone();
						decremented_schedule.cycle = decremented_schedule.cycle.saturating_sub(1);

						// put the task in map for next execution if cycle more than once
						if decremented_schedule.cycle > 0 {
							self.repetitive_tasks
								.entry(index + decremented_schedule.frequency)
								.or_insert(vec![])
								.push((schedule_id, shard_id, decremented_schedule));
						}
						self.error_count.remove(&schedule_id);
					},
					Err(error) => {
						log::error!(
							"Error occured while executing task {:?}: {}",
							schedule_id,
							error
						);

						let is_terminated =
							self.report_schedule_invalid(schedule_id, false, block_id, shard_id);

						// if not terminated keep add task with added frequency
						if !is_terminated {
							self.repetitive_tasks
								.entry(index + schedule.frequency)
								.or_insert(vec![])
								.push((schedule_id, shard_id, schedule));
						}
					},
				}
			}
			self.last_block_height = index;
		}

		Ok(())
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
			// get the external blockchain's block number
			let Ok(status) = self.rosetta_client.status().await else {
				continue;
			};
			let current_block = status.index;
			// update last block height if never set before
			if self.last_block_height == 0 {
				self.last_block_height = current_block;
			}

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
