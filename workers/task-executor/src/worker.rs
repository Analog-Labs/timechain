use crate::{BlockHeight, TaskExecutorParams, TW_LOG};
use anyhow::{Context, Result};
use codec::{Decode, Encode};
use dotenv::dotenv;
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, StreamExt};
use graphql_client::{GraphQLQuery, Response as GraphQLResponse};
use reqwest::header;
use rosetta_client::{create_wallet, EthereumExt, Wallet};
use sc_client_api::{Backend, BlockchainEvents};
use serde_json::Value;
use sp_api::{HeaderT, ProvideRuntimeApi};
use sp_core::offchain::STORAGE_PREFIX;
use sp_core::sr25519;
use sp_keystore::KeystorePtr;
use sp_runtime::offchain::OffchainStorage;
use sp_runtime::traits::Block;
use std::env;
use std::{
	collections::{HashSet, VecDeque},
	marker::PhantomData,
	sync::Arc,
};
use time_primitives::abstraction::{OCWPayload, OCW_MAX_TRY};
use time_primitives::ShardId;
use time_primitives::{
	Function, FunctionResult, OCWSkdData, ScheduleCycle, ScheduleStatus, TaskId, TaskSchedule,
	TimeApi, TimeId, TssSignature, OCW_SKD_KEY, TIME_KEY_TYPE,
};
use time_worker::TssRequest;
use timechain_integration::query::{collect_data, CollectData};

pub struct Task {
	tss: mpsc::Sender<TssRequest>,
	wallet: Arc<Wallet>,
}

impl Task {
	pub fn new(tss: mpsc::Sender<TssRequest>, wallet: Arc<Wallet>) -> Self {
		Self { tss, wallet }
	}

	/// Fetches and executes contract call for a given schedule_id
	async fn execute_function(&self, function: &Function) -> Result<FunctionResult> {
		match function {
			// If the task function is an Ethereum contract
			// call, call it and send for signing
			Function::EVMViewWithoutAbi {
				address,
				function_signature,
				input,
			} => {
				let data = self.wallet.eth_view_call(address, function_signature, input).await?;
				let result = match data.result {
					Value::Array(val) => val
						.iter()
						.filter_map(|x| x.as_str())
						.map(|x| x.to_string())
						.collect::<Vec<String>>(),
					v => vec![v.to_string()],
				};
				Ok(FunctionResult::EVMViewWithoutAbi { result })
			},
		}
	}

	/// Encode call response and send data for tss signing process
	async fn tss_sign(
		&self,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: ScheduleCycle,
		result: &FunctionResult,
	) -> Result<TssSignature> {
		let data = bincode::serialize(&result).context("Failed to serialize task")?;
		let (tx, rx) = oneshot::channel();
		self.tss
			.clone()
			.send(TssRequest {
				request_id: (task_id, cycle),
				shard_id,
				data,
				tx,
			})
			.await?;
		Ok(rx.await?)
	}

	async fn submit_to_timegraph(
		&self,
		target_block_number: BlockHeight,
		result: &FunctionResult,
		collection: String,
	) -> Result<()> {
		// Add data into collection (user must have Collector role)
		// @collection: collection hashId
		// @cycle: time-chain block number
		// @block: target network block number
		// @task_id: task associated with data
		// @task_counter: for repeated task it's incremented on every run
		// @tss: TSS signature
		// @data: data to add into collection

		let FunctionResult::EVMViewWithoutAbi { result } = result;
		let variables = collect_data::Variables {
			collection,
			block: target_block_number as i64,
			// unused field
			task_id: 0,
			// unused field
			cycle: 0,
			data: result.clone(),
		};
		dotenv().ok();
		let url =
			env::var("TIMEGRAPH_GRAPHQL_URL").context("Unable to get timegraph graphql url")?;
		let ssk = env::var("SSK").context("Unable to get timegraph ssk")?;

		// Build the GraphQL request
		let request = CollectData::build_query(variables);
		// Execute the GraphQL request
		let client = reqwest::Client::new();
		let response = client
			.post(url)
			.json(&request)
			.header(header::AUTHORIZATION, ssk)
			.send()
			.await
			.context("error in post request to timegraph")?;
		let data = response
			.json::<GraphQLResponse<collect_data::ResponseData>>()
			.await
			.context("Failed to parse timegraph response")?
			.data
			.context("timegraph migrate collect status fail: No reponse")?;
		log::info!("timegraph migrate collect status: {:?}", data.collect.status);
		Ok(())
	}

	async fn execute(
		self,
		target_block: BlockHeight,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: ScheduleCycle,
		task: TaskSchedule,
	) -> Result<TssSignature> {
		let result = self.execute_function(&task.function).await?;
		let signature = self.tss_sign(shard_id, task_id, cycle, &result).await?;
		self.submit_to_timegraph(target_block, &result, task.hash.clone()).await?;
		Ok(signature)
	}
}

pub struct TaskExecutor<B: Block, BE, R> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	sign_data_sender: mpsc::Sender<TssRequest>,
	kv: KeystorePtr,
	wallet: Arc<Wallet>,
	running_tasks: HashSet<TaskId>,
}

impl<B, BE, R> TaskExecutor<B, BE, R>
where
	B: Block,
	BE: Backend<B> + 'static,
	R: BlockchainEvents<B> + ProvideRuntimeApi<B>,
	R::Api: TimeApi<B>,
{
	pub async fn new(params: TaskExecutorParams<B, BE, R>) -> Result<Self> {
		let TaskExecutorParams {
			_block,
			backend,
			runtime,
			sign_data_sender,
			kv,
			connector_url,
			connector_blockchain,
			connector_network,
		} = params;
		// create rosetta client and get chain configuration
		let wallet =
			create_wallet(connector_blockchain, connector_network, connector_url, None).await?;
		Ok(Self {
			_block,
			backend,
			runtime,
			sign_data_sender,
			kv,
			wallet: Arc::new(wallet),
			running_tasks: Default::default(),
		})
	}

	fn account_id(&self) -> Option<TimeId> {
		let Some(id) = self.public_key() else {
			return None;
		};
		TimeId::decode(&mut id.as_ref()).ok()
	}

	/// Returns the public key for the worker if one was set.
	fn public_key(&self) -> Option<sr25519::Public> {
		let keys = self.kv.sr25519_public_keys(TIME_KEY_TYPE);
		if keys.is_empty() {
			log::warn!(target: TW_LOG, "No time key found, please inject one.");
			return None;
		}
		Some(keys[0])
	}

	async fn start_tasks(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let Some(account) = self.account_id() else {
			anyhow::bail!("No account id found");
		};

		let status = self.wallet.status().await?;
		let block_height = status.index;

		let shards = self.runtime.runtime_api().get_shards(block_id, account).unwrap();
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id).unwrap();
			for (task_id, cycle) in tasks {
				if self.running_tasks.contains(&task_id) {
					continue;
				}
				let task_descr =
					self.runtime.runtime_api().get_task(block_id, task_id).unwrap().unwrap();
				if block_height >= task_descr.trigger(cycle) {
					self.running_tasks.insert(task_id);
					let task = Task::new(self.sign_data_sender.clone(), self.wallet.clone());
					let backend = self.backend.clone();
					tokio::task::spawn(async move {
						let ocw_storage = backend.offchain_storage();

						let status = match task
							.execute(block_height, shard_id, task_id, cycle, task_descr)
							.await
						{
							Ok(signature) => ScheduleStatus::Ok(shard_id, signature),
							Err(e) => ScheduleStatus::Err(e.to_string()),
						};

						update_schedule_ocw(task_id, cycle, status, ocw_storage);
					});
					// self.update_schedule_ocw(1, 1, ScheduleStatus::Ok(1, [0u8; 64]));
				}
			}
		}
		Ok(())
	}

	pub async fn run(&mut self) {
		let mut finality_notifications = self.runtime.finality_notification_stream();
		while let Some(notification) = finality_notifications.next().await {
			if let Err(err) = self.start_tasks(notification.header.hash()).await {
				log::error!("error processing tasks: {}", err);
			}
		}
	}
}

fn update_schedule_ocw<B>(
	task_id: TaskId,
	cycle: ScheduleCycle,
	status: ScheduleStatus,
	storage: Option<B>,
) where
	B: OffchainStorage,
{
	let skd_data = OCWSkdData::new(task_id, cycle, status);
	let payload = OCWPayload::OCWSkd(skd_data);
	update_ocw_storage(OCW_SKD_KEY, payload, 0, storage);
}

fn update_ocw_storage<B>(ocw_key: &[u8], payload: OCWPayload, retry_num: u8, ocw_storage: Option<B>)
where
	B: OffchainStorage,
{
	let Some(mut ocw_storage) = ocw_storage else{
			log::error!("cant get offchain storage");
			return;
		};
	// if let Some(mut ocw_storage) = self.backend.offchain_storage() {
	let old_value = ocw_storage.get(STORAGE_PREFIX, ocw_key);

	let mut ocw_vec = match old_value.clone() {
		Some(mut data) => {
			//remove this unwrap
			let mut bytes: &[u8] = &mut data;
			let inner_data: VecDeque<OCWPayload> = Decode::decode(&mut bytes).unwrap();
			inner_data
		},
		None => Default::default(),
	};

	ocw_vec.push_back(payload.clone());
	let encoded_data = Encode::encode(&ocw_vec);

	let is_data_stored =
		ocw_storage.compare_and_set(STORAGE_PREFIX, ocw_key, old_value.as_deref(), &encoded_data);

	if !is_data_stored {
		if retry_num < OCW_MAX_TRY {
			let retry_num = retry_num + 1;
			update_ocw_storage(ocw_key, payload, retry_num, Some(ocw_storage));
		} else {
			log::error!("Failed to store data in ocw_storage");
		}
	} else {
		log::info!("tss key stored in ocw");
	}
}
