use super::{Message, Network, PeerId, PROTOCOL_NAME};
use anyhow::Result;
use futures::channel::oneshot;
use futures::{Future, Stream};
use sc_network::config::{IncomingRequest, RequestResponseConfig};
use sc_network::multiaddr::multihash::MultihashGeneric as Multihash;
use sc_network::request_responses::OutgoingResponse;
use sc_network::{IfDisconnected, NetworkRequest, NetworkSigner, PublicKey};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

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

pub struct SubstrateNetwork<N> {
	network: N,
	peer_id: PeerId,
}

impl<N> SubstrateNetwork<N>
where
	N: NetworkRequest + NetworkSigner,
{
	pub fn new(network: N) -> Result<Self> {
		let public_key = network.sign_with_local_identity([])?.public_key;
		let peer_id = public_key.clone().try_into_ed25519()?.to_bytes();
		Ok(Self { network, peer_id })
	}
}

impl<N: NetworkRequest + Send + Sync + 'static> Network for SubstrateNetwork<N> {
	fn peer_id(&self) -> PeerId {
		self.peer_id
	}

	fn send(
		&self,
		peer_id: PeerId,
		msg: Message,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
		let bytes = bincode::serialize(&msg).unwrap();
		let (tx, rx) = oneshot::channel();
		let peer_id = sc_network::PeerId::from_public_key(
			&sc_network::config::ed25519::PublicKey::try_from_bytes(&peer_id).unwrap().into(),
		);
		self.network.start_request(
			peer_id,
			PROTOCOL_NAME.into(),
			bytes,
			tx,
			IfDisconnected::TryConnect,
		);
		Box::pin(async move {
			let response = rx.await??;
			Ok(bincode::deserialize(&response)?)
		})
	}
}

fn parse_peer_id(peer: sc_network::PeerId) -> Option<PeerId> {
	let mh = Multihash::from(peer);
	if mh.code() != 0 {
		return None;
	}
	let p = PublicKey::try_decode_protobuf(mh.digest()).ok()?;
	let p = p.try_into_ed25519().ok()?;
	Some(p.to_bytes())
}

pub struct SubstrateNetworkAdapter(async_channel::Receiver<IncomingRequest>);

impl SubstrateNetworkAdapter {
	pub fn new(rx: async_channel::Receiver<IncomingRequest>) -> Self {
		Self(rx)
	}
}

impl Stream for SubstrateNetworkAdapter {
	type Item = (PeerId, Message);

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		loop {
			match Pin::new(&mut self.0).poll_next(cx) {
				Poll::Ready(Some(IncomingRequest {
					peer,
					payload,
					pending_response,
				})) => {
					// Don't try to do anything other than reply immediately as
					// substrate will close the substream.
					let _ = pending_response.send(OutgoingResponse {
						result: Ok(vec![]),
						reputation_changes: vec![],
						sent_feedback: None,
					});
					let Some(peer) = parse_peer_id(peer) else {
						continue;
					};
					if let Ok(msg) = bincode::deserialize(&payload) {
						return Poll::Ready(Some((peer, msg)));
					}
				},
				Poll::Ready(None) => return Poll::Ready(None),
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}
