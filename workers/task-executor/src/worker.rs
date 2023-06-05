use crate::{TaskExecutorParams, TW_LOG};
use anyhow::{Context, Result};
use codec::Decode;
use frame_system::{
	offchain::{SigningTypes, SubmitTransaction},
	pallet, Error,
};
use futures::channel::mpsc::Sender;
use rosetta_client::{
	create_client,
	types::{CallRequest, CallResponse},
	BlockchainConfig, Client,
};
use sc_client_api::Backend;
use serde_json::json;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as _;
use sp_core::{crypto::AccountId32, hashing::keccak_256};
use sp_keyring::{sr25519::sr25519::Pair, AccountKeyring};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{collections::HashSet, marker::PhantomData, sync::Arc, time::Duration};
use subxt::tx::PairSigner;
use subxt::{OnlineClient, PolkadotConfig};

use time_db::{feed::Model, fetch_event::Model as FEModel, DatabaseConnection};
use time_primitives::{
	abstraction::{Function, ScheduleStatus},
	KeyId, TimeApi, TimeId, KEY_TYPE,
};
use tokio::runtime::Runtime;

#[subxt::subxt(
	runtime_metadata_path = "../../artifacts/testnet-metadata.scale",
	derive_for_all_types = "PartialEq, Clone"
)]
pub mod timechain {}

pub struct TimechainSubmitter {
	pub signer: PairSigner<PolkadotConfig, Pair>,
	pub api: OnlineClient<PolkadotConfig>,
	pub account: AccountId32,
}

pub struct TaskExecutor<B, BE, R, A> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	_account_id: PhantomData<A>,
	sign_data_sender: Sender<(u64, u64, [u8; 32])>,
	kv: KeystorePtr,
	tasks: HashSet<u64>,
	db: DatabaseConnection,
	chain_config: BlockchainConfig,
	chain_client: Client,
}

impl<B, BE, R, A> TaskExecutor<B, BE, R, A>
where
	B: Block,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	A: codec::Codec,
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
			db,
			chain_config,
			chain_client,
		})
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

	// pub fn offchain_worker<T: frame_system::Config + frame_system::offchain::SigningTypes>(status:ScheduleStatus, key: KeyId) {
	// 	let signer: frame_system::offchain::Signer<_, _, frame_system::offchain::ForAll> = frame_system::offchain::Signer::<T, T::AuthorityId>::all_accounts();

	// 	let results = signer.send_signed_transaction(|_account| {
	// 		pallet::Call::update_schedule { status, key }
	// 	});

	// 	for (acc, res) in &results {
	// 		match res {
	// 			Ok(()) => log::info!("[{:?}]: submit transaction success.", acc.id),
	// 			Err(e) => log::error!("[{:?}]: submit transaction failure. Reason: {:?}", acc.id, e),
	// 		}
	// 	}
	// }

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
		self.sign_data_sender.clone().try_send((shard_id, task_id, hash))?;
		self.tasks.insert(id);
		if *shard.collector() == account {
			self.runtime
				.runtime_api()
				.update_schedule_by_key(block_id, ScheduleStatus::Completed, id)?
				.map_err(|err| anyhow::anyhow!("{:?}", err))?;
		}

		let api =  OnlineClient::<PolkadotConfig>::new().await?;
		    // Build a storage query to access account information.
			// let account = AccountKeyring::Alice.to_account_id().into();
			let storage_query = timechain::storage().task_schedule().schedule_storage(1);
		
		let x = api.storage().at_latest().await?.fetch(&storage_query).await?;
		log::info!("\n\n\n==subxt api =====> {:?}\n\n",x.unwrap());

		Ok(true)
	}

	async fn process_tasks_for_block(&mut self, block_id: <B as Block>::Hash) -> Result<()> {
		let task_schedules = self
			.runtime
			.runtime_api()
			.get_task_schedule(block_id)?
			.map_err(|err| anyhow::anyhow!("{:?}", err))?;
		for (id, schedule) in &task_schedules {
			if !self.tasks.contains(id) {
				let metadata = self
					.runtime
					.runtime_api()
					.get_task_metadat_by_key(block_id, schedule.task_id.0)?
					.map_err(|err| anyhow::anyhow!("{:?}", err))?;
				let Some(task) = metadata else {
					log::info!("task schedule id have no metadata, Removing task from Schedule list");
					self.runtime.runtime_api().update_schedule_by_key(
						block_id,
						ScheduleStatus::Invalid,
						*id,
					)?.map_err(|err| anyhow::anyhow!("{:?}", err))?;
					// TODO: Remove task from schedule list
					return Ok(());
				};
				let shard_id = schedule.shard_id;
				match &task.function {
					// If the task function is an Ethereum contract
					// call, call it and send for signing
					Function::EthereumViewWithoutAbi {
						address,
						function_signature,
						input: _,
						output: _,
					} => {
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
								schedule.task_id.0,
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
								let _ =
									time_db::write_fetch_event(&mut self.db, fetch_record).await;
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
		}
		Ok(())
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
