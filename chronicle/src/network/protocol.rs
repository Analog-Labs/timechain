use super::{Message, Network, NetworkConfig, PeerId, PROTOCOL_NAME};
use anyhow::Result;
use futures::channel::mpsc;
use futures::{Future, FutureExt, SinkExt};
use peernet::{Endpoint, NotificationHandler, Protocol, ProtocolHandler};
use std::pin::Pin;
use std::time::Duration;
use tracing::field;

pub struct TssEndpoint {
	endpoint: Endpoint,
}

struct TssProtocol;

impl Protocol for TssProtocol {
	const ID: u16 = 0;
	const REQ_BUF: usize = 1024;
	const RES_BUF: usize = 1024;
	type Request = Message;
	type Response = Message;
}

#[derive(Clone)]
struct TssProtocolHandler {
	tx: mpsc::Sender<(PeerId, Message)>,
}

impl TssProtocolHandler {
	pub fn new(tx: mpsc::Sender<(PeerId, Message)>) -> Self {
		Self { tx }
	}
}

impl NotificationHandler<TssProtocol> for TssProtocolHandler {
	fn notify(&self, peer: peernet::PeerId, req: Message) -> Result<()> {
		let mut tx = self.tx.clone();
		tokio::spawn(async move {
			tx.send((*peer.as_bytes(), req)).await.ok();
		});
		Ok(())
	}
}

impl TssEndpoint {
	pub async fn new(config: NetworkConfig, tx: mpsc::Sender<(PeerId, Message)>) -> Result<Self> {
		let mut builder = ProtocolHandler::builder();
		builder.register_notification_handler(TssProtocolHandler::new(tx));
		let handler = builder.build();

		let mut builder = Endpoint::builder(PROTOCOL_NAME.as_bytes().to_vec());
		builder.secret(config.secret);
		builder.handler(handler);
		builder.republish_interval(Duration::from_secs(60 * 5));
		builder.publish_ttl(Duration::from_secs(60 * 5 * 4));
		builder.relay_map(None);
		let endpoint = builder.build().await?;
		let peer_id = endpoint.peer_id();
		loop {
			tracing::info!(
				peer_id = field::display(peer_id),
				"waiting for peer id to be registered",
			);
			let Ok(addr) = endpoint.resolve(peer_id).await else {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			};
			if addr != endpoint.addr().await?.info {
				tokio::time::sleep(Duration::from_secs(1)).await;
				continue;
			}
			tracing::info!(peer_id = field::display(peer_id), "peer id registered",);
			break;
		}
		Ok(Self { endpoint })
	}
}

impl Network for TssEndpoint {
	fn peer_id(&self) -> PeerId {
		*self.endpoint.peer_id().as_bytes()
	}

	fn format_peer_id(&self, peer: PeerId) -> String {
		peernet::PeerId::from_bytes(&peer).unwrap().to_string()
	}

	fn send(&self, peer: PeerId, msg: Message) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		let endpoint = self.endpoint.clone();
		async move {
			let peer = peernet::PeerId::from_bytes(&peer)?;
			endpoint.notify::<TssProtocol>(peer, &msg).await
		}
		.boxed()
	}
}
