#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use dotenvy::dotenv;
use futures::channel::mpsc::Sender;
use sc_client_api::Backend;
use serde_json::from_str;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::Backend as SpBackend;
use sp_runtime::{generic::BlockId, traits::Block};
use std::error::Error;
use std::{marker::PhantomData, sync::Arc, thread};
use time_primitives::abstraction::{Function, Task};
use time_primitives::TimeApi;
use time_worker::kv::TimeKeyvault;
use tokio::{sync::Mutex, time};
use web3::contract::{Contract, Options};
use web3::types::{Address, H160, U256};

// use worker_aurora::{self, establish_connection, get_on_chain_data};

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

	async fn call_contract_function(
		address: String,
		abi: String,
		method: String,
	) -> Result<(), Box<dyn Error>> {
		dotenv().ok();
		log::info!("-------- inside call contract function");
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
		let contract_abi = from_str(&abi).unwrap();
		let contract_address = Address::from(address.parse::<H160>().unwrap());

		// Create a new contract instance using the ABI and address
		let contract = Contract::new(web3.eth(), contract_address, contract_abi);

		// Call the "getGreeting" function on the contract instance
		let greeting: String =
			contract.query(method.as_str(), (), None, Options::default(), None).await?;

		println!("The greeting is: {}", greeting);

		Ok(())
	}

	pub(crate) async fn run(&mut self) {
		let sign_data_sender_clone = self.sign_data_sender.clone();
		let delay = time::Duration::from_secs(10);
		loop {
			let keys = self.kv.public_keys();
			if !keys.is_empty() {
				if let Ok(at) = self.backend.blockchain().last_finalized() {
					let at = BlockId::Hash(at);
					if let Ok(metadata_result) = self.runtime.runtime_api().get_task_metadata(&at) {
						log::info!("\n\nmetadata_result --> {:?}\n\n", metadata_result);
						match metadata_result {
							Ok(metadata) => {
								metadata.iter().for_each(|task| {
									match &task.function {
										Function::EthereumContract {
											address,
											abi,
											function,
											input: _,
											output: _,
										} => {
											let _result = Self::call_contract_function(
												address.to_string(),
												abi.to_string(),
												function.to_string(),
											);

											log::info!(
												"\n\n\n {:?} \n{:?}\n{:?}\n",
												address,
												abi.to_string(),
												function.to_string()
											);
										},
										Function::EthereumApi {
											function: _,
											input: _,
											output: _,
										} => {
											todo!()
										},
									};
								});
							},
							Err(e) => {
								log::info!("No metadata found for block {:?}: {:?}", at, e);
							},
						}
					} else {
						log::error!("Failed to get task metadata for block {:?}", at);
					}
				} else {
					log::error!("Blockchain is empty");
				}
				thread::sleep(delay);
			}
		}
	}
}
