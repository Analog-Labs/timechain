#![allow(clippy::type_complexity)]
use crate::WorkerParams;
use codec::Decode;
use futures::{
	channel::mpsc::{Receiver, Sender},
	FutureExt, StreamExt,
};
use rosetta_client::{create_client, types::CallRequest};
use sc_client_api::Backend;
use serde_json::json;
use sp_api::ProvideRuntimeApi;
use sp_io::hashing::keccak_256;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block;
use std::{error::Error, marker::PhantomData, sync::Arc};
use time_primitives::{abstraction::EthTxValidation, TimeApi};

#[allow(unused)]
/// Our structure, which holds refs to everything we need to operate
pub struct EventWorker<B: Block, A, R, BE> {
	pub(crate) runtime: Arc<R>,
	pub(crate) backend: Arc<BE>,
	_block: PhantomData<B>,
	sign_data_sender: Sender<(u64, [u8; 32])>,
	tx_data_receiver: Receiver<Vec<u8>>,
	kv: KeystorePtr,
	pub accountid: PhantomData<A>,
	connector_url: Option<String>,
	connector_blockchain: Option<String>,
	connector_network: Option<String>,
}

impl<B, A, R, BE> EventWorker<B, A, R, BE>
where
	B: Block,
	A: codec::Codec,
	R: ProvideRuntimeApi<B>,
	BE: Backend<B>,
	R::Api: TimeApi<B, A>,
{
	pub(crate) fn new(worker_params: WorkerParams<B, A, R, BE>) -> Self {
		let WorkerParams {
			runtime,
			sign_data_sender,
			tx_data_receiver,
			kv,
			backend,
			_block,
			accountid: _,
			connector_url,
			connector_blockchain,
			connector_network,
		} = worker_params;

		EventWorker {
			runtime,
			sign_data_sender,
			tx_data_receiver,
			kv,
			backend,
			_block: PhantomData,
			accountid: PhantomData,
			connector_url,
			connector_blockchain,
			connector_network,
		}
	}

	pub async fn process_tx_validation_req(
		&self,
		eth_tx_data: EthTxValidation,
	) -> Result<[u8; 32], Box<dyn Error>> {
		let (config, client) =
			create_client(eth_tx_data.blockchain, eth_tx_data.network, eth_tx_data.url)
				.await
				.unwrap();

		let call_req = CallRequest {
			network_identifier: config.network(),
			method: format!("{}--transaction_receipt", eth_tx_data.tx_id),
			parameters: json!({}),
		};

		let received_tx = eth_tx_data.tx_id.strip_prefix("0x").unwrap_or(&eth_tx_data.tx_id);
		let received_contract_address = eth_tx_data
			.contract_address
			.strip_prefix("0x")
			.unwrap_or(&eth_tx_data.contract_address);

		let receipt = client.call(&call_req).await.unwrap();
		let result = receipt.result;

		let receipt_tx_hash = result["transactionHash"].as_str().unwrap_or("");
		let receipt_tx_hash = receipt_tx_hash.strip_prefix("0x").unwrap_or(receipt_tx_hash);

		let receipt_contract_address = result["to"].as_str().unwrap_or("");
		let receipt_contract_address =
			receipt_contract_address.strip_prefix("0x").unwrap_or(receipt_contract_address);

		if receipt_tx_hash == received_tx && receipt_contract_address == received_contract_address {
			let tx_hash_and_contract = format!("{receipt_tx_hash}-{receipt_contract_address}");
			let hashed_val = keccak_256(tx_hash_and_contract.as_bytes());
			log::info!("Contract execution is valid");
			Ok(hashed_val)
		} else {
			Err("Invalid tx hash or contract address".into())
		}
	}

	pub(crate) async fn run(&mut self) {
		let mut sign_data_sender_clone = self.sign_data_sender.clone();

		loop {
			futures::select! {
				data = self.tx_data_receiver.next().fuse() => {
					let Some(data) = data else{
						continue;
					};
					let Ok(eth_tx_validation) = EthTxValidation::decode(&mut &data[..]) else {
						continue;
					};
					match self.process_tx_validation_req(eth_tx_validation).await{
						Ok(keccak_hash) => {
							// process validated successfully
							sign_data_sender_clone.try_send((1, keccak_hash)).unwrap();
							log::info!("sent data for signing");
						}
						Err(e) => {
							//error occured while matching tx data
							log::error!("Error occured while verifying trasnaction {:?}", e);
						}
					}
				}
			}
		}
	}
}
