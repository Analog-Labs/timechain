#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use bincode::serialize;
use core::time;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use jsonrpsee::{
	core::Error as JsonRpseeError,
	types::{error::CallError, ErrorObject},
};
use sc_client_api::Backend;

use serde_json::from_str;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_core::{sr25519, Pair};
use sp_io::hashing::{keccak_256, keccak_512};

use sp_runtime::{generic::BlockId, traits::Block};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};
use time_primitives::{abstraction::Function, rpc::SignRpcPayload, TimeApi};
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time::sleep};
use web3::{
	contract::{Contract, Options},
	types::H160,
};

#[derive(Debug, thiserror::Error)]
/// Top-level error type for the RPC handler
pub enum Error {
	/// Provided signature verification failed
	#[error("Provided signature verification failed")]
	SigVerificationFailure,
	/// Time key is not yet injected into node
	#[error("No time key found")]
	TimeKeyNotFound,
}

// The error codes returned by jsonrpc.
pub enum ErrorCode {
	/// Returned when signature in given SignRpcPayload failed to verify
	SigFailure = 1,
	NoTimeKey = 2,
}

impl From<Error> for ErrorCode {
	fn from(error: Error) -> Self {
		match error {
			Error::SigVerificationFailure => ErrorCode::SigFailure,
			Error::TimeKeyNotFound => ErrorCode::NoTimeKey,
		}
	}
}

impl From<Error> for JsonRpseeError {
	fn from(error: Error) -> Self {
		let message = error.to_string();
		let code = ErrorCode::from(error);
		JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
			code as i32,
			message,
			None::<()>,
		)))
	}
}

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct TaskExecutor<B: Block, R, BE> {
	pub(crate) backend: Arc<BE>,
	pub(crate) runtime: Arc<R>,
	_block: PhantomData<B>,
	sign_data_sender: Arc<Mutex<Sender<(u64, [u8; 32])>>>,
	kv: TimeKeyvault,
}

impl<B, R, BE> TaskExecutor<B, R, BE>
where
	B: Block,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, R, BE>) -> Self {
		let WorkerParams {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block,
		} = worker_params;

		TaskExecutor {
			backend,
			runtime,
			sign_data_sender,
			kv,
			_block: PhantomData,
		}
	}

	pub fn hash_keccak_256(input: &[u8]) -> [u8; 32] {
		keccak_256(input)
	}

	pub fn hash_keccak_512(input: &[u8]) -> [u8; 64] {
		keccak_512(input)
	}

	async fn call_contract_and_send_for_sign(
		&self,
		address: String,
		abi: String,
		method: String,
		shard_id: u64,
	) -> Result<(), Box<dyn std::error::Error>> {
		dotenv().ok();
		let infura_url = std::env::var("INFURA_URL").expect("INFURA_URL must be set");

		let websocket_result = web3::transports::WebSocket::new(&infura_url).await;
		let websocket = match websocket_result {
			Ok(websocket) => websocket,
			Err(_) => web3::transports::WebSocket::new(&infura_url)
				.await
				.expect("Failed to create default websocket"),
		};
		let web3 = web3::Web3::new(websocket);

		// Load the contract ABI and address
		let contract_abi = match from_str(&abi) {
			Ok(abi) => abi,
			Err(e) => {
				log::warn!("Error parsing contract ABI:  {}", e);
				return Err("Failed to parse contract ABI".into());
			},
		};
		let contract_address = match address.parse::<H160>() {
			Ok(parsed_address) => parsed_address,
			Err(parse_error) => {
				// handle the error here, for example by printing a message and exiting the program
				log::warn!("Error parsing contract address: {}", parse_error);
				std::process::exit(1);
			},
		};

		// Create a new contract instance using the ABI and address
		let contract = Contract::new(web3.eth(), contract_address, contract_abi);

		let message: String =
			contract.query(method.as_str(), (), None, Options::default(), None).await?;

		if let Ok(task_in_bytes) = serialize(&message) {
			let hash = Self::hash_keccak_256(&task_in_bytes);
			let phrase = "owner word vocal dose decline sunset battle example forget excite gentle waste//1//time";
			let keypair = sr25519::Pair::from_string(&phrase, None).unwrap();
			let raw_signature = keypair.sign(&hash);

			let signature = format!(
				"[{}]",
				raw_signature
					.0
					.iter()
					.map(|b| format!("{:02x}", b))
					.collect::<Vec<String>>()
					.join(",")
			);

			let ser = serialize(&signature).unwrap();
			let signaturee = Self::hash_keccak_512(&ser);
			let keys = self.kv.public_keys();

			if keys.len() != 1 {
				return Err(Error::TimeKeyNotFound.into());
			}

			let payload = SignRpcPayload::new(shard_id, hash, signaturee);
			log::info!("\n\n\n--> payload verify {:?}\n", payload.verify(keys[0].clone()));
			if payload.verify(keys[0].clone()) {
				match self.sign_data_sender.lock().await.try_send((payload.group_id, hash)) {
					Ok(()) => {
						log::info!("Connector successfully send event to -- channel")
					},
					Err(_) => {
						log::info!("Connector failed to send event to channel")
					},
				}
				Ok(())
			} else {
				Err(Error::SigVerificationFailure.into())
			}
		} else {
			log::info!("Failed to serialize task: {:?}", message);
			Ok(())
		}
	}

	async fn process_tasks_for_block(
		&self,
		block_id: BlockId<B>,
		map: &mut HashMap<u64, String>,
	) -> Result<(), Box<dyn std::error::Error>> {
		// let tasks_schedule = self.runtime.runtime_api().get_task_schedule(&block_id)?;
		// Get the task schedule for the current block
		let tasks_schedule = match self.runtime.runtime_api().get_task_schedule(&block_id) {
			Ok(task_schedule) => task_schedule,
			Err(_e) => Ok({
				log::error!("Failed to get task schedule for block {:?}", block_id);
				Vec::new() // Return an empty vector as the default value
			}),
		};

		match tasks_schedule {
			Ok(task_schedule) => {
				for schedule_task in task_schedule.iter() {
					let task_id = schedule_task.task_id.0;
					let shard_id = 123; //schedule_task.shard_id;

					match map.insert(task_id, "hash".to_string()) {
						Some(old_value) =>
							println!("The key already existed with the value {}", old_value),
						None => {
							println!("The key didn't exist and was inserted key {}.", task_id);
							let metadata_result = self
								.runtime
								.runtime_api()
								.get_task_metadat_by_key(&block_id, task_id)?;

							match metadata_result {
								Ok(metadata) => {
									for task in metadata.iter() {
										match &task.function {
											// If the task function is an Ethereum contract call,
											// call it and send for signing
											Function::EthereumContract {
												address,
												abi,
												function,
												input: _,
												output: _,
											} => {
												let _result =
													Self::call_contract_and_send_for_sign(
														self,
														address.to_string(),
														abi.to_string(),
														function.to_string(),
														shard_id,
													)
													.await;
											},
											Function::EthereumApi {
												function: _,
												input: _,
												output: _,
											} => {
												todo!()
											},
										};
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
						},
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
		let mut map: HashMap<u64, String> = HashMap::new();
		loop {
			// Get the public keys from the Key-Value store to check key is set
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				// Get the last finalized block from the blockchain
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					let at = BlockId::Hash(at);
					match self.process_tasks_for_block(at, &mut map).await {
						Ok(_) => (),
						Err(e) => {
							log::error!("Failed to process tasks for block {:?}: {:?}", at, e);
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
