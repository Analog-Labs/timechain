use crate::network::NetworkConfig;
use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::substrate::Substrate;
use crate::tasks::executor::{TaskExecutor, TaskExecutorParams};
use crate::tasks::spawner::{TaskSpawner, TaskSpawnerParams};
use anyhow::{Context, Result};
use futures::channel::mpsc;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::request_responses::IncomingRequest;
use sc_network::{NetworkRequest, NetworkSigner};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::path::PathBuf;
use std::sync::Arc;
use tc_subxt::{AccountInterface, SubxtClient};
use time_primitives::{
	BlockHash, BlockTimeApi, MembersApi, Network, ShardsApi, SubmitTransactionApi, TasksApi,
};
use tracing::{event, span, Level};

mod network;
mod shards;
mod substrate;
mod tasks;
mod gmp;

pub use crate::network::protocol_config;

pub const TW_LOG: &str = "chronicle";

pub struct ChronicleConfig {
	pub secret: Option<PathBuf>,
	pub bind_port: Option<u16>,
	pub pkarr_relay: Option<String>,
	pub blockchain: Network,
	pub network: String,
	pub url: String,
	pub timechain_keyfile: PathBuf,
	pub keyfile: Option<PathBuf>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
}

pub struct ChronicleParams<B: Block, C, R, N> {
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub tx_pool: OffchainTransactionPoolFactory<B>,
	pub network: Option<(N, async_channel::Receiver<IncomingRequest>)>,
	pub config: ChronicleConfig,
}

pub async fn run_chronicle<B, C, R, N>(params: ChronicleParams<B, C, R, N>) -> Result<()>
where
	B: Block<Hash = BlockHash>,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B> + ShardsApi<B> + TasksApi<B> + BlockTimeApi<B> + SubmitTransactionApi<B>,
	N: NetworkRequest + NetworkSigner + Send + Sync + 'static,
{
	let secret = if let Some(path) = params.config.secret {
		Some(
			std::fs::read(path)
				.context("secret doesn't exist")?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid secret"))?,
		)
	} else {
		None
	};
	let (network, net_request) = if let Some((network, incoming)) = params.network {
		crate::network::create_substrate_network(network, incoming).await?
	} else {
		crate::network::create_iroh_network(NetworkConfig {
			secret,
			bind_port: params.config.bind_port,
			relay: params.config.pkarr_relay,
		})
		.await?
	};
	let peer_id = network.peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		?peer_id,
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "PeerId {:?}", peer_id);

	let (tss_tx, tss_rx) = mpsc::channel(10);
	let subxt_client =
		SubxtClient::new("ws://127.0.0.1:9944", Some(&params.config.timechain_keyfile)).await?;
	tracing::info!("Nonce at creation {:?}", subxt_client.nonce());
	let substrate =
		Substrate::new(true, params.tx_pool, params.client, params.runtime, subxt_client);

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
		task_spawner,
		substrate: substrate.clone(),
	});

	let time_worker = TimeWorker::new(TimeWorkerParams {
		network,
		task_executor,
		substrate,
		tss_request: tss_rx,
		net_request,
	});

	time_worker.run(&span).await;
	Ok(())
}
