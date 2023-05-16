use crate::{ConnectorWorkerParams, TW_LOG};
use anyhow::Result;
use codec::Decode;
use futures::channel::mpsc::Sender;
use rosetta_client::{
	create_client,
	types::{BlockRequest, PartialBlockIdentifier},
	BlockchainConfig, Client,
};
use sc_client_api::Backend;
use serde_json::Value;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as _;
use sp_core::hashing::keccak_256;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc, time::Duration};
use time_db::DatabaseConnection;
use time_primitives::{TimeApi, TimeId, KEY_TYPE};

/// Our structure, which holds refs to everything we need to operate
pub struct ConnectorWorker<B: Block, A, R, BE> {
	_block: PhantomData<B>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	_account_id: PhantomData<A>,
	kv: KeystorePtr,
	sign_data_sender: Sender<(u64, [u8; 32])>,
	db: DatabaseConnection,
	chain_config: BlockchainConfig,
	chain_client: Client,
}

impl<B, A, R, BE> ConnectorWorker<B, A, R, BE>
where
	B: Block,
	A: codec::Codec,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A>,
{
	pub async fn new(params: ConnectorWorkerParams<B, A, R, BE>) -> Result<Self> {
		let ConnectorWorkerParams {
			runtime,
			sign_data_sender,
			kv,
			backend,
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
			kv,
			sign_data_sender,
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
			TimeId::decode(&mut keys[0].as_ref()).ok()
		}
	}

	async fn get_swap_data_from_db(&mut self) -> Vec<[u8; 32]> {
		let mut tasks_from_db_bytes: Vec<[u8; 32]> = Vec::new();
		if let Ok(tasks_from_db) = time_db::read_on_chain_data(&mut self.db, 10).await {
			for task in tasks_from_db.iter() {
				if let Ok(task_in_bytes) = bincode::serialize(task) {
					tasks_from_db_bytes.push(keccak_256(&task_in_bytes));
				} else {
					log::error!("Failed to serialize task: {:?}", task);
				}
			}
		}
		tasks_from_db_bytes
	}

	async fn get_latest_block_event(&self) -> Result<()> {
		//TODO! take this from runtime
		let contract_address = "0x678ea0447843f69805146c521afcbcc07d6e28a2";

		let network_status = self.chain_client.network_status(self.chain_config.network()).await?;

		let block_req = BlockRequest {
			network_identifier: self.chain_config.network(),
			block_identifier: PartialBlockIdentifier {
				index: Some(network_status.current_block_identifier.index),
				hash: None,
			},
		};

		let block_data = self.chain_client.block(&block_req).await?;

		let empty_vec: Vec<Value> = vec![];
		if let Some(data) = block_data.block {
			for tx in data.transactions {
				if let Some(metadata) = tx.metadata {
					let receipts = metadata["receipt"]["logs"].as_array().unwrap_or(&empty_vec);
					let filtered_receipt = receipts.iter().filter(|log| {
						let address = log["address"].as_str().unwrap_or("");
						address == contract_address
					});

					for log in filtered_receipt {
						if let Ok(log_in_bytes) = bincode::serialize(&log) {
							let hash = keccak_256(&log_in_bytes);
							// if this node is collector for given shard - we submit for siging
							// TODO: change hardcoded 1 to actual shard id
							// TODO: stabilize no unwraps and blind indexing!
							let at = self.backend.blockchain().last_finalized().unwrap();

							if let Some(my_key) = self.account_id() {
								let current_shard = self
									.runtime
									.runtime_api()
									.get_shards(at)
									.unwrap_or(vec![])
									.into_iter()
									//get this 1 as shard from runtime tbd later
									.find(|(s, _)| *s == 1);

								if let Some(shard) = current_shard {
									if shard.1.collector() == &my_key {
										match self.sign_data_sender.clone().try_send((1, hash)) {
											Ok(()) => {
												log::info!(
													"Connector successfully send event to channel"
												)
											},
											Err(_) => {
												log::info!(
													"Connector failed to send event to channel"
												)
											},
										}
									} else {
										log::info!("Failed to serialize log: {:?}", log);
									}
								} else {
									log::error!(
										target: TW_LOG,
										"connector-worker no matching shard found"
									);
								}
							} else {
								log::error!(target: TW_LOG, "Failed to construct account");
							}
						} else {
							log::info!("Failed to serialize log: {:?}", log);
						}
					}
				}
			}
		};
		Ok(())
	}

	pub async fn run(&mut self) {
		loop {
			if self.account_id().is_some() {
				// Get swap data from db and send it to time-worker
				let tasks_in_byte = self.get_swap_data_from_db().await;
				if !tasks_in_byte.is_empty() {
					for task in tasks_in_byte.iter() {
						let result = self.sign_data_sender.try_send((1, *task));
						match result {
							Ok(_) => log::warn!("sign_data_sender_clone ok"),
							Err(_) => log::warn!("sign_data_sender_clone err"),
						}
					}

					// Get latest block event from Uniswap v2 and send it to time-worker
					if let Err(e) = self.get_latest_block_event().await {
						log::error!(
							"XXXXXXXX-Error occured while fetching block data {e:?}-XXXXXXXX"
						);
					}
				}
			}
			tokio::time::sleep(Duration::from_secs(3)).await;
		}
	}
}
