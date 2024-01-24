use crate::network::{Message, Network, NetworkConfig, PeerId};
use crate::shards::{TimeWorker, TimeWorkerParams};
use crate::substrate::Substrate;
use crate::tasks::executor::{TaskExecutor, TaskExecutorParams};
use crate::tasks::spawner::{TaskSpawner, TaskSpawnerParams};
use anyhow::Result;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::request_responses::IncomingRequest;
use sc_network::{NetworkRequest, NetworkSigner};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::path::PathBuf;
use std::sync::Arc;
use tc_subxt::SubxtClient;
use time_primitives::{
	AccountInterface, BlockHash, BlockTimeApi, MembersApi, NetworkId, NetworksApi, Runtime,
	ShardsApi, SubmitTransactionApi, TasksApi,
};
use tracing::{event, span, Level};

mod gmp;
mod network;
mod shards;
mod substrate;
mod tasks;

pub use crate::network::create_iroh_network;
pub use crate::network::protocol_config;

pub const TW_LOG: &str = "chronicle";

pub struct ChronicleConfig {
	pub secret: Option<PathBuf>,
	pub bind_port: Option<u16>,
	pub pkarr_relay: Option<String>,
	pub network_id: NetworkId,
	pub url: String,
	pub timechain_keyfile: PathBuf,
	pub keyfile: Option<PathBuf>,
	pub timegraph_url: Option<String>,
	pub timegraph_ssk: Option<String>,
}

impl ChronicleConfig {
	pub fn network_config(&self) -> NetworkConfig {
		NetworkConfig {
			secret: self.secret.clone(),
			bind_port: self.bind_port,
			relay: self.pkarr_relay.clone(),
		}
	}
}

pub struct ChronicleParams<B: Block, C, R, N> {
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub tx_pool: OffchainTransactionPoolFactory<B>,
	pub network: Option<(N, async_channel::Receiver<IncomingRequest>)>,
	pub config: ChronicleConfig,
}

pub async fn run_node_with_chronicle<B, C, R, N>(params: ChronicleParams<B, C, R, N>) -> Result<()>
where
	B: Block<Hash = BlockHash>,
	C: BlockchainEvents<B> + HeaderBackend<B> + 'static,
	R: ProvideRuntimeApi<B> + Send + Sync + 'static,
	R::Api: MembersApi<B>
		+ NetworksApi<B>
		+ ShardsApi<B>
		+ TasksApi<B>
		+ BlockTimeApi<B>
		+ SubmitTransactionApi<B>,
	N: NetworkRequest + NetworkSigner + Send + Sync + 'static,
{
	let (network, net_request) = if let Some((network, incoming)) = params.network {
		crate::network::create_substrate_network(network, incoming).await?
	} else {
		crate::network::create_iroh_network(params.config.network_config()).await?
	};

	let subxt_client =
		SubxtClient::new("ws://127.0.0.1:9944", Some(&params.config.timechain_keyfile)).await?;
	tracing::info!("Nonce at creation {:?}", subxt_client.nonce());
	let substrate =
		Substrate::new(true, params.tx_pool, params.client, params.runtime, subxt_client);

	run_chronicle(params.config, network, net_request, substrate).await
}

pub async fn run_chronicle(
	config: ChronicleConfig,
	network: Arc<dyn Network>,
	net_request: BoxStream<'static, (PeerId, Message)>,
	substrate: impl Runtime,
) -> Result<()> {
	let peer_id = network.peer_id();
	let span = span!(
		target: TW_LOG,
		Level::INFO,
		"run_chronicle",
		?peer_id,
	);
	event!(target: TW_LOG, parent: &span, Level::INFO, "PeerId {:?}", peer_id);

	let (chain, subchain) = substrate
		.get_network(config.network_id)?
		.ok_or(anyhow::anyhow!("Network Id not supported"))?;
	let chain: time_primitives::Network = chain.parse()?;

	let (tss_tx, tss_rx) = mpsc::channel(10);
	let task_spawner_params = TaskSpawnerParams {
		tss: tss_tx,
		blockchain: chain,
		network: subchain,
		url: config.url,
		keyfile: config.keyfile,
		timegraph_url: config.timegraph_url,
		timegraph_ssk: config.timegraph_ssk,
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
		network: chain,
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
