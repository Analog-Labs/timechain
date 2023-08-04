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
use sc_client_api::HeaderBackend;
use serde_json::{json, Value};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as _;
use sp_core::offchain::STORAGE_PREFIX;
use sp_core::sr25519;
use sp_keystore::KeystorePtr;
use sp_runtime::offchain::OffchainStorage;
use sp_runtime::traits::Block;
use std::env;
use std::{
	collections::{HashMap, HashSet, VecDeque},
	fmt,
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};
use time_primitives::crypto::Signature;
use time_primitives::ShardId;
use time_primitives::{
	abstraction::{Function, FunctionResult, OCWSigData, OCWSkdData, ScheduleStatus},
	ScheduleCycle, SignatureData, TaskId, TaskSchedule, TimeApi, TimeId, OCW_SIG_KEY, OCW_SKD_KEY,
	TIME_KEY_TYPE,
};
use time_worker::TssRequest;
use timechain_integration::query::{collect_data, CollectData};

#[derive(Clone)]
pub struct TaskExecutor<B, BE, R, A, BN> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	_account_id: PhantomData<A>,
	_block_number: PhantomData<BN>,
	sign_data_sender: mpsc::Sender<TssRequest>,
	kv: KeystorePtr,
	rosetta_chain_config: BlockchainConfig,
	rosetta_client: Client,
	last_block_height: BlockHeight,
	running_tasks: HashSet<(TaskId, ScheduleCycle)>,
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
			rosetta_chain_config,
			rosetta_client,
			last_block_height: 0,
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
				let method = format!("{address}-{function_signature}-call");
				let request = CallRequest {
					network_identifier: self.rosetta_chain_config.network(),
					method,
					parameters: json!(input),
				};
				let data = self.rosetta_client.call(&request).await?;
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
	) -> Result<SignatureData> {
		let data = bincode::serialize(&result).context("Failed to serialize task")?;
		let (tx, rx) = oneshot::channel();
		self.sign_data_sender
			.send(TssRequest {
				request_id: (task_id, cycle),
				shard_id,
				data,
				tx,
			})
			.await?;
		Ok(rx.await?)
	}

	fn update_schedule_ocw(&self, task_id: TaskId, cycle: ScheduleCycle, status: ScheduleStatus) {
		let skd_data = OCWSkdData::new(task_id, cycle, status);
		if let Some(mut ocw_storage) = self.backend.offchain_storage() {
			let old_value = ocw_storage.get(STORAGE_PREFIX, OCW_SKD_KEY);

			let mut ocw_vec = match old_value.clone() {
				Some(mut data) => {
					//remove this unwrap
					let mut bytes: &[u8] = &mut data;
					let inner_data: VecDeque<Vec<u8>> = Decode::decode(&mut bytes).unwrap();
					inner_data
				},
				None => Default::default(),
			};

			ocw_vec.push_back(skd_data.encode());
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

	async fn submit_to_timegraph(
		&self,
		block_id: <B as Block>::Hash,
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

		let local_block_number: i64 = match self.backend.blockchain().number(block_id) {
			//will not fail since block number is u32
			Ok(Some(val)) => val.to_string().parse().unwrap(),
			Ok(None) => 0.into(),
			Err(e) => {
				log::warn!("Error occurred: {:?}", e);
				0.into()
			},
		};

		let FunctionResult::EVMViewWithoutAbi { result } = result;
		let variables = collect_data::Variables {
			collection,
			block: target_block_number as i64,
			cycle: local_block_number,
			// unused field
			task_id: 0,
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

	async fn execute_task(
		&self,
		task_block: <B as Block>::Hash,
		target_block: BlockHeight,
		shard_id: ShardId,
		task_id: TaskId,
		cycle: ScheduleCycle,
		task: &TaskSchedule<A>,
	) -> Result<SignatureData> {
		let result = self.execute_function(&task.function).await?;
		let signature = self.tss_sign(shard_id, task_id, cycle, &result).await?;
		self.submit_to_timegraph(task_block, target_block, &result, task.hash.clone())
			.await?;
		Ok(signature)
	}

	async fn process_tasks_for_block(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let Some(account) = self.account_id() else {
			anyhow::bail!("No account id found");
		};
		let status =
			self.rosetta_client.network_status(self.rosetta_chain_config.network()).await?;
		let block_height = status.current_block_identifier.index;

		let shards = self.runtime.runtime_api().get_shards(block_id, account).unwrap();
		for shard_id in shards {
			let tasks = self.runtime.runtime_api().get_shard_tasks(block_id, shard_id).unwrap();
			for (task_id, cycle) in tasks {
				if self.running_tasks.contains(&(task_id, cycle)) {
					continue;
				}
				let task = self.runtime.runtime_api().get_task(block_id, task_id).unwrap().unwrap();
				if task.is_triggered(block_height) {
					self.running_tasks.insert(task_id);
					tokio::task::spawn(async move {
						let status = match self
							.execute_task(block_id, block_height, shard_id, task_id, cycle, &task)
							.await
						{
							Ok(signature) => ScheduleStatus::Ok(shard_id, signature),
							Err(e) => ScheduleStatus::Err(e.to_string()),
						};
						self.update_schedule_ocw(task_id, cycle, status);
					});
				}
			}
		}
		Ok(())
	}

	pub async fn run(&mut self) {}
}
