use super::{Message, Network, NetworkConfig, PeerId, PROTOCOL_NAME};
use anyhow::Result;
use futures::channel::mpsc;
use futures::{Future, FutureExt, SinkExt};
use p2p::{Endpoint, NotificationHandler, Protocol, ProtocolHandler};
use std::pin::Pin;

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
	fn notify(&self, peer: p2p::PeerId, req: Message) -> Result<()> {
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
		if let Some(secret) = config.secret {
			builder.secret(secret);
		}
		if let Some(port) = config.bind_port {
			builder.port(port);
		}
		if let Some(relay) = config.relay {
			builder.relay(relay.parse()?);
		} else {
			builder.localhost_relay();
		};
		builder.handler(handler);
		let endpoint = builder.build().await?;
		Ok(Self { endpoint })
	}
}

impl Network for TssEndpoint {
	fn peer_id(&self) -> PeerId {
		*self.endpoint.peer_id().as_bytes()
	}

	fn send(&self, peer: PeerId, msg: Message) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		let endpoint = self.endpoint.clone();
		async move {
			let peer = p2p::PeerId::from_bytes(&peer).unwrap();
			endpoint.notify::<TssProtocol>(&peer, &msg).await
		}
		.boxed()
	}
}
