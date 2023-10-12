use crate::shards::{NetworkConfig, TimeWorker, TimeWorkerParams};
use crate::substrate::Substrate;
use crate::tasks::executor::{TaskExecutor, TaskExecutorParams};
use crate::tasks::spawner::{TaskSpawner, TaskSpawnerParams};
use anyhow::Result;
use futures::channel::mpsc;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::request_responses::IncomingRequest;
use sc_network::{NetworkRequest, NetworkSigner};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::traits::Block;
use std::sync::Arc;
use time_primitives::{
	BlockHash, BlockTimeApi, MembersApi, Network, PublicKey, ShardsApi, TasksApi, TIME_KEY_TYPE,
};
use tracing::{event, span, Level};

mod shards;
mod substrate;
mod tasks;

pub use crate::shards::protocol_config;

pub const TW_LOG: &str = "chronicle";

pub struct ChronicleConfig {
	pub secret: Option<[u8; 32]>,
	pub bind_port: Option<u16>,
	pub pkarr_relay: Option<String>,
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
	pub network: Option<(N, async_channel::Receiver<IncomingRequest>)>,
	pub config: ChronicleConfig,
}

pub async fn run_chronicle<B, C, R, N>(params: ChronicleParams<B, C, R, N>) -> Result<()>
where
	B: Block<Hash = BlockHash>,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B> + BlockTimeApi<B>,
	N: NetworkRequest + NetworkSigner + Send + Sync + 'static,
{
	let (network, net_request) = crate::shards::network(
		params.network,
		NetworkConfig {
			secret: params.config.secret,
			bind_port: params.config.bind_port,
			relay: params.config.pkarr_relay,
		},
	)
	.await?;
	let peer_id = network.peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		peer_id = format!("{peer_id:?}"),
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "PeerId {:?}", peer_id);

	let public_key: PublicKey = loop {
		if let Some(pubkey) = params.keystore.sr25519_public_keys(TIME_KEY_TYPE).into_iter().next()
		{
			break pubkey.into();
		}
		event!(target: TW_LOG, parent: &span, Level::INFO, "Waiting for public key to be inserted");
		tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
	};

	let (tss_tx, tss_rx) = mpsc::channel(10);
	let substrate = Substrate::new(
		true,
		params.keystore,
		params.tx_pool,
		params.client.clone(),
		params.runtime,
	);

	let task_spawner_params = TaskSpawnerParams {
		tss: tss_tx,
		blockchain: params.config.blockchain,
		network: params.config.network,
		url: params.config.url,
		keyfile: params.config.keyfile,
		timegraph_url: params.config.timegraph_url,
		timegraph_ssk: params.config.timegraph_ssk,
		substrate: substrate.clone(),
	};
	let task_spawner = loop {
		match TaskSpawner::new(task_spawner_params.clone()).await {
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
		network: params.config.blockchain,
		public_key: public_key.clone(),
		task_spawner,
		substrate: substrate.clone(),
	});

	let time_worker = TimeWorker::new(TimeWorkerParams {
		network,
		task_executor,
		substrate,
		public_key,
		tss_request: tss_rx,
		net_request,
	});

	time_worker.run(&span).await;
	Ok(())
}
