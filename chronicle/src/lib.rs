use crate::network::{TimeWorker, TimeWorkerParams};
use crate::task_executor::{Task, TaskExecutor, TaskExecutorParams, TaskSpawnerParams};
use crate::tx_submitter::TransactionSubmitter;
use futures::channel::mpsc;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::config::{IncomingRequest, RequestResponseConfig};
use sc_network::{NetworkRequest, NetworkSigner};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::Block;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use time_primitives::{
	BlockTimeApi, MembersApi, Network, PublicKey, ShardsApi, TasksApi, TIME_KEY_TYPE,
};
use tracing::{event, span, Level};

mod network;
mod task_executor;
#[cfg(test)]
mod tests;
mod tx_submitter;

pub const TW_LOG: &str = "chronicle";

/// chronicle protocol name suffix.
pub const PROTOCOL_NAME: &str = "/chronicle/1";

pub fn protocol_config(tx: async_channel::Sender<IncomingRequest>) -> RequestResponseConfig {
	RequestResponseConfig {
		name: PROTOCOL_NAME.into(),
		fallback_names: vec![],
		max_request_size: 1024 * 1024,
		max_response_size: 0,
		request_timeout: Duration::from_secs(3),
		inbound_queue: Some(tx),
	}
}

pub struct ChronicleConfig {
	pub blockchain: Network,
	pub network: String,
	pub url: String,
	pub keyfile: Option<String>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
}

pub struct ChronicleParams<B: Block, C, R, N> {
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub keystore: KeystorePtr,
	pub tx_pool: OffchainTransactionPoolFactory<B>,
	pub network: N,
	pub tss_requests: async_channel::Receiver<IncomingRequest>,
	pub config: ChronicleConfig,
}

pub async fn run_chronicle<B, C, R, N>(params: ChronicleParams<B, C, R, N>)
where
	B: Block + 'static,
	C: BlockchainEvents<B> + HeaderBackend<B> + Send + Sync + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B> + BlockTimeApi<B>,
	N: NetworkRequest + NetworkSigner,
{
	let public_key = params.network.sign_with_local_identity([]).unwrap().public_key;
	let peer_id = public_key.clone().try_into_ed25519().unwrap().to_bytes();
	let libp2p_peer_id = public_key.to_peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		peer_id = format!("{:?}", peer_id),
		libp2p_peer_id = libp2p_peer_id.to_string(),
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "Peer identity bytes: {:?}", peer_id);

	let public_key: PublicKey = loop {
		if let Some(pubkey) = params.keystore.sr25519_public_keys(TIME_KEY_TYPE).into_iter().next()
		{
			break pubkey.into();
		}
		event!(target: TW_LOG, parent: &span, Level::INFO, "Waiting for public key to be inserted");
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	};

	let (tx, rx) = mpsc::channel(10);
	let tx_submitter = TransactionSubmitter::new(
		true,
		params.keystore,
		params.tx_pool,
		params.client.clone(),
		params.runtime.clone(),
	);

	let task_spawner_params = TaskSpawnerParams {
		_marker: PhantomData,
		tss: tx,
		blockchain: params.config.blockchain,
		network: params.config.network,
		url: params.config.url,
		keyfile: params.config.keyfile,
		timegraph_url: params.config.timegraph_url,
		timegraph_ssk: params.config.timegraph_ssk,
		runtime: params.runtime.clone(),
		tx_submitter: tx_submitter.clone(),
	};
	let task_spawner = loop {
		match Task::new(task_spawner_params.clone()).await {
			Ok(task_spawner) => break task_spawner,
			Err(error) => {
				event!(
					target: TW_LOG,
					parent: &span,
					Level::INFO,
					"Initializing wallet returned an error {:?}, retrying in one second",
					error
				);
				tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
			},
		}
	};

	let task_executor = TaskExecutor::new(TaskExecutorParams {
		_block: PhantomData,
		runtime: params.runtime.clone(),
		network: params.config.blockchain,
		public_key: public_key.clone(),
		task_spawner,
	});

	let time_worker = TimeWorker::new(TimeWorkerParams {
		_block: PhantomData,
		runtime: params.runtime.clone(),
		client: params.client.clone(),
		network: params.network,
		task_executor,
		tx_submitter,
		public_key,
		peer_id,
		tss_request: rx,
		protocol_request: params.tss_requests,
	});

	time_worker.run(&span).await;
}
