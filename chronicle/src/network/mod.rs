use self::protocol::TssEndpoint;
use self::substrate::{SubstrateNetwork, SubstrateNetworkAdapter};
use anyhow::Result;
use futures::channel::mpsc;
use futures::stream::BoxStream;
use futures::{Future, StreamExt};
use sc_network::request_responses::IncomingRequest;
use sc_network::{NetworkRequest, NetworkSigner};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use time_primitives::{BlockNumber, ShardId, TssId};

mod protocol;
mod substrate;

pub use self::substrate::protocol_config;
pub use time_primitives::PeerId;

pub type TssMessage = tss::TssMessage<TssId>;

pub const PROTOCOL_NAME: &str = "/analog-labs/chronicle/1";

#[derive(Default)]
pub struct NetworkConfig {
	pub secret: Option<PathBuf>,
	pub relay: Option<String>,
	pub bind_port: Option<u16>,
}

#[derive(Deserialize, Serialize)]
pub struct Message {
	pub shard_id: ShardId,
	pub block_number: BlockNumber,
	pub payload: TssMessage,
}

pub trait Network: Send + Sync + 'static {
	fn peer_id(&self) -> PeerId;

	fn send(
		&self,
		peer_id: PeerId,
		msg: Message,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

impl Network for Arc<dyn Network> {
	fn peer_id(&self) -> PeerId {
		self.deref().peer_id()
	}

	fn send(
		&self,
		peer_id: PeerId,
		msg: Message,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		self.deref().send(peer_id, msg)
	}
}

pub async fn create_substrate_network<N: NetworkRequest + NetworkSigner + Send + Sync + 'static>(
	network: N,
	incoming: async_channel::Receiver<IncomingRequest>,
) -> Result<(Arc<dyn Network>, BoxStream<'static, (PeerId, Message)>)> {
	let network = Arc::new(SubstrateNetwork::new(network)?) as Arc<dyn Network + Send + Sync>;
	let incoming = SubstrateNetworkAdapter::new(incoming).boxed();
	Ok((network, incoming))
}

pub async fn create_iroh_network(
	config: NetworkConfig,
) -> Result<(Arc<dyn Network>, BoxStream<'static, (PeerId, Message)>)> {
	let (net_tx, net_rx) = mpsc::channel(10);
	let network =
		Arc::new(TssEndpoint::new(config, net_tx).await?) as Arc<dyn Network + Send + Sync>;
	let incoming = net_rx.boxed();
	Ok((network, incoming))
}
