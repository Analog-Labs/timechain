use anyhow::Result;
use sc_client_api::{BlockchainEvents, HeaderBackend};
use sc_network::request_responses::IncomingRequest;
use sc_network::{NetworkRequest, NetworkSigner};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;
use time_primitives::{
	BlockHash, BlockTimeApi, MembersApi, NetworksApi, ShardsApi, SubmitTransactionApi, TasksApi,
};

mod network;
mod runtime;

pub use network::protocol_config;

pub struct ChronicleParams<B: Block, C, R, N> {
	pub client: Arc<C>,
	pub runtime: Arc<R>,
	pub tx_pool: OffchainTransactionPoolFactory<B>,
	pub network: Option<(N, async_channel::Receiver<IncomingRequest>)>,
	pub config: chronicle::ChronicleConfig,
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
		network::create_substrate_network(network, incoming).await?
	} else {
		chronicle::create_iroh_network(params.config.network_config()).await?
	};

	let subxt_client = tc_subxt::SubxtClient::with_keyfile(
		"ws://127.0.0.1:9944",
		&params.config.timechain_keyfile,
	)
	.await?;
	let substrate =
		runtime::Substrate::new(true, params.tx_pool, params.client, params.runtime, subxt_client);

	chronicle::run_chronicle(params.config, network, net_request, substrate).await
}
