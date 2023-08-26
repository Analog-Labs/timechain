#![allow(clippy::large_enum_variant)]
use crate::dkg::{Dkg, DkgAction, DkgMessage};
use crate::roast::{Roast, RoastAction, RoastMessage};
use frost_evm::keys::{KeyPackage, PublicKeyPackage};
use frost_evm::{Identifier, Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod dkg;
pub mod roast;
#[cfg(test)]
mod tests;

enum TssState<I> {
	Dkg {
		dkg: Dkg,
	},
	Roast {
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		signing_sessions: BTreeMap<I, Roast>,
	},
}

#[derive(Clone)]
pub enum TssAction<I, P> {
	Send(Vec<(P, TssMessage<I>)>),
	PublicKey(VerifyingKey),
	Signature(I, Signature),
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub enum TssMessage<I> {
	Dkg { msg: DkgMessage },
	Roast { id: I, msg: RoastMessage },
}

impl<I: std::fmt::Display> std::fmt::Display for TssMessage<I> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Dkg { msg } => write!(f, "dkg {}", msg),
			Self::Roast { id, msg } => write!(f, "roast {} {}", id, msg),
		}
	}
}

fn peer_to_frost(peer: impl std::fmt::Display) -> Identifier {
	Identifier::derive(peer.to_string().as_bytes()).expect("non zero")
}

/// Tss state machine.
pub struct Tss<I, P> {
	peer_id: P,
	frost_id: Identifier,
	frost_to_peer: BTreeMap<Identifier, P>,
	threshold: u16,
	is_coordinator: bool,
	state: TssState<I>,
}

impl<I, P> Tss<I, P>
where
	I: Clone + Ord + std::fmt::Display,
	P: Clone + Ord + std::fmt::Display,
{
	pub fn new(peer_id: P, members: BTreeSet<P>, threshold: u16) -> Self {
		debug_assert!(members.contains(&peer_id));
		let frost_id = peer_to_frost(&peer_id);
		let frost_to_peer: BTreeMap<_, _> =
			members.into_iter().map(|peer| (peer_to_frost(&peer), peer)).collect();
		let index = frost_to_peer.iter().position(|(id, _)| *id == frost_id).unwrap();
		let is_coordinator = index < (frost_to_peer.len() - threshold as usize + 1);
		let dkg = Dkg::new(frost_id, frost_to_peer.keys().cloned().collect(), threshold);
		log::debug!(
			"{} initialize {}/{} coordinator = {}",
			peer_id,
			threshold,
			frost_to_peer.len(),
			is_coordinator
		);
		Self {
			peer_id,
			frost_id,
			frost_to_peer,
			threshold,
			is_coordinator,
			state: TssState::Dkg { dkg },
		}
	}

	pub fn peer_id(&self) -> &P {
		&self.peer_id
	}

	fn frost_to_peer(&self, frost: &Identifier) -> P {
		self.frost_to_peer.get(frost).unwrap().clone()
	}

	pub fn total_nodes(&self) -> usize {
		self.frost_to_peer.len()
	}

	pub fn threshold(&self) -> usize {
		self.threshold as _
	}

	pub fn on_message(&mut self, peer_id: P, msg: TssMessage<I>) {
		log::debug!("{} on_message {} {}", self.peer_id, peer_id, msg);
		if self.peer_id == peer_id {
			log::error!("{} dropping message from self", self.peer_id);
			return;
		}
		let frost_id = peer_to_frost(&peer_id);
		if !self.frost_to_peer.contains_key(&frost_id) {
			log::error!("{} dropping message from {}", self.peer_id, peer_id);
			return;
		}
		match (&mut self.state, msg) {
			(TssState::Dkg { dkg }, TssMessage::Dkg { msg }) => {
				dkg.on_message(frost_id, msg);
			},
			(TssState::Roast { signing_sessions, .. }, TssMessage::Roast { id, msg }) => {
				if let Some(session) = signing_sessions.get_mut(&id) {
					session.on_message(frost_id, msg);
				} else {
					log::error!("no signing session for {}", id);
				}
			},
			(_, msg) => {
				log::error!("invalid state ({}, {}, {})", self.peer_id, peer_id, msg);
			},
		}
	}

	pub fn sign(&mut self, id: I, data: Vec<u8>) {
		log::debug!("{} sign {}", self.peer_id, id);
		match &mut self.state {
			TssState::Roast {
				key_package,
				public_key_package,
				signing_sessions,
			} => {
				let roast = Roast::new(
					self.threshold,
					key_package.clone(),
					public_key_package.clone(),
					data,
					self.is_coordinator,
				);
				signing_sessions.insert(id, roast);
			},
			TssState::Dkg { .. } => {
				log::error!("not ready to sign");
			},
		}
	}

	pub fn next_action(&mut self) -> Option<TssAction<I, P>> {
		match &mut self.state {
			TssState::Dkg { dkg } => {
				match dkg.next_action()? {
					DkgAction::Send(msgs) => {
						return Some(TssAction::Send(
							msgs.into_iter()
								.map(|(peer, msg)| {
									(self.frost_to_peer(&peer), TssMessage::Dkg { msg })
								})
								.collect(),
						));
					},
					DkgAction::Broadcast(msg) => {
						return Some(TssAction::Send(
							self.frost_to_peer
								.values()
								.filter(|peer| **peer != self.peer_id)
								.cloned()
								.map(|peer| (peer, TssMessage::Dkg { msg: msg.clone() }))
								.collect(),
						));
					},
					DkgAction::Complete(key_package, public_key_package) => {
						let public_key =
							VerifyingKey::new(public_key_package.group_public().to_element());
						self.state = TssState::Roast {
							key_package,
							public_key_package,
							signing_sessions: Default::default(),
						};
						return Some(TssAction::PublicKey(public_key));
					},
					DkgAction::Failure(error) => {
						log::error!("dkg failed with error {}", error);
						return None;
					},
				};
			},
			TssState::Roast { signing_sessions, .. } => {
				let session_ids: Vec<_> = signing_sessions.keys().cloned().collect();
				for id in session_ids {
					let session = signing_sessions.get_mut(&id).unwrap();
					while let Some(action) = session.next_action() {
						let (peers, send_to_self, msg) = match action {
							RoastAction::Broadcast(msg) => {
								let peers = self
									.frost_to_peer
									.keys()
									.filter(|peer| **peer != self.frost_id)
									.copied()
									.collect();
								(peers, true, msg)
							},
							RoastAction::Send(peer, msg) => {
								if peer == self.frost_id {
									(vec![], true, msg)
								} else {
									(vec![peer], false, msg)
								}
							},
							RoastAction::SendMany(all_peers, msg) => {
								let peers: Vec<_> = all_peers
									.iter()
									.filter(|peer| **peer != self.frost_id)
									.copied()
									.collect();
								let send_to_self = peers.len() != all_peers.len();
								(peers, send_to_self, msg)
							},
							RoastAction::Complete(signature) => {
								signing_sessions.remove(&id);
								return Some(TssAction::Signature(id, signature));
							},
						};
						if send_to_self {
							session.on_message(self.frost_id, msg.clone());
						}
						if !peers.is_empty() {
							return Some(TssAction::Send(
								peers
									.into_iter()
									.map(|peer| {
										(
											self.frost_to_peer(&peer),
											TssMessage::Roast {
												id: id.clone(),
												msg: msg.clone(),
											},
										)
									})
									.collect(),
							));
						}
					}
				}
			},
		}
		None
	}
}
