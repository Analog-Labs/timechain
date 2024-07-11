#![allow(clippy::large_enum_variant)]
use crate::dkg::{Dkg, DkgAction, DkgMessage};
use crate::roast::{Roast, RoastAction, RoastMessage};
use anyhow::Result;
use frost_evm::keys::{KeyPackage, PublicKeyPackage, SecretShare};
use frost_evm::Scalar;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tracing::field;

pub use frost_evm::frost_core::keys::sum_commitments;
pub use frost_evm::frost_secp256k1::Signature as ProofOfKnowledge;
pub use frost_evm::keys::{SigningShare, VerifiableSecretSharingCommitment};
pub use frost_evm::schnorr::SigningKey;
pub use frost_evm::{Identifier, Signature, VerifyingKey};

mod dkg;
mod roast;
#[cfg(test)]
mod tests;

#[allow(dead_code)]
enum TssState<I> {
	Dkg(Dkg),
	Roast {
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		signing_sessions: BTreeMap<I, Roast>,
	},
	Failed,
}

#[derive(Clone)]
pub enum TssAction<I, P> {
	Send(Vec<(P, TssMessage<I>)>),
	Commit(VerifiableSecretSharingCommitment, ProofOfKnowledge),
	Ready(SigningShare, VerifiableSecretSharingCommitment, VerifyingKey),
	Signature(I, [u8; 32], Signature),
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub enum TssMessage<I> {
	Dkg { msg: DkgMessage },
	Roast { id: I, msg: RoastMessage },
}

impl<I> TssMessage<I> {
	pub fn is_response(&self) -> bool {
		match self {
			Self::Roast { msg, .. } => msg.is_response(),
			_ => false,
		}
	}
}

impl<I: std::fmt::Display> std::fmt::Display for TssMessage<I> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Dkg { msg } => write!(f, "dkg {}", msg),
			Self::Roast { id, msg } => write!(f, "roast {} {}", id, msg),
		}
	}
}

pub trait ToFrostIdentifier {
	fn to_frost(&self) -> Identifier;
}

pub fn construct_proof_of_knowledge(
	peer: impl ToFrostIdentifier,
	coefficients: &[Scalar],
	commitment: &VerifiableSecretSharingCommitment,
) -> Result<ProofOfKnowledge> {
	Ok(frost_evm::frost_core::keys::dkg::compute_proof_of_knowledge(
		peer.to_frost(),
		coefficients,
		commitment,
		OsRng,
	)?)
}

pub fn verify_proof_of_knowledge(
	peer: impl ToFrostIdentifier,
	commitment: &VerifiableSecretSharingCommitment,
	proof_of_knowledge: ProofOfKnowledge,
) -> Result<()> {
	Ok(frost_evm::frost_core::keys::dkg::verify_proof_of_knowledge(
		peer.to_frost(),
		commitment,
		proof_of_knowledge,
	)?)
}

/// Tss state machine.
pub struct Tss<I, P> {
	peer_id: P,
	frost_id: Identifier,
	frost_to_peer: BTreeMap<Identifier, P>,
	threshold: u16,
	coordinators: BTreeSet<Identifier>,
	state: TssState<I>,
	committed: bool,
}

impl<I, P> Tss<I, P>
where
	I: Clone + Ord + std::fmt::Display,
	P: Clone + Ord + std::fmt::Display + ToFrostIdentifier,
{
	pub fn new(
		peer_id: P,
		members: BTreeSet<P>,
		threshold: u16,
		recover: Option<(SigningShare, VerifiableSecretSharingCommitment)>,
	) -> Self {
		debug_assert!(members.contains(&peer_id));
		let frost_id = peer_id.to_frost();
		let frost_to_peer: BTreeMap<_, _> =
			members.into_iter().map(|peer| (peer.to_frost(), peer)).collect();
		let members: BTreeSet<_> = frost_to_peer.keys().copied().collect();
		let coordinators: BTreeSet<_> =
			members.iter().copied().take(members.len() - threshold as usize + 1).collect();
		let is_coordinator = coordinators.contains(&frost_id);
		tracing::info!(
			peer_id = field::display(peer_id.clone()),
			"initialize {}/{} coordinator = {}",
			threshold,
			members.len(),
			is_coordinator,
		);
		let committed = recover.is_some();
		Self {
			peer_id,
			frost_id,
			frost_to_peer,
			threshold,
			coordinators,
			state: if let Some((signing_share, commitment)) = recover {
				let secret_share = SecretShare::new(frost_id, signing_share, commitment.clone());
				let key_package = KeyPackage::try_from(secret_share).expect("valid signing share");
				let public_key_package =
					PublicKeyPackage::from_commitment(&members, &commitment).unwrap();
				TssState::Roast {
					key_package,
					public_key_package,
					signing_sessions: Default::default(),
				}
			} else {
				TssState::Dkg(Dkg::new(frost_id, members, threshold))
			},
			committed,
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

	pub fn committed(&self) -> bool {
		self.committed
	}

	pub fn on_message(&mut self, peer_id: P, msg: TssMessage<I>) -> Result<()> {
		tracing::info!(
			peer_id = field::display(self.peer_id.clone()),
			"on_message from peer_id={peer_id} msg={msg}",
		);
		if self.peer_id == peer_id {
			anyhow::bail!("{} received message from self", self.peer_id);
		}
		let frost_id = peer_id.to_frost();
		if !self.frost_to_peer.contains_key(&frost_id) {
			anyhow::bail!("{} received message unknown peer {}", self.peer_id, peer_id);
		}
		match (&mut self.state, msg) {
			(TssState::Dkg(dkg), TssMessage::Dkg { msg }) => {
				dkg.on_message(frost_id, msg);
			},
			(TssState::Roast { signing_sessions, .. }, TssMessage::Roast { id, msg }) => {
				if let Some(session) = signing_sessions.get_mut(&id) {
					session.on_message(frost_id, msg);
				} else {
					tracing::info!("invalid signing session {}", id);
				}
			},
			(_, msg) => {
				anyhow::bail!("unexpected message {}", msg);
			},
		}
		Ok(())
	}

	pub fn on_commit(&mut self, commitment: VerifiableSecretSharingCommitment) {
		tracing::info!(peer_id = field::display(self.peer_id.clone()), "commit",);
		match &mut self.state {
			TssState::Dkg(dkg) => {
				dkg.on_commit(commitment);
				self.committed = true;
			},
			_ => tracing::error!("unexpected commit"),
		}
	}

	fn get_or_insert_session(&mut self, id: I) -> Option<&mut Roast> {
		match &mut self.state {
			TssState::Roast {
				key_package,
				public_key_package,
				signing_sessions,
				..
			} => Some(signing_sessions.entry(id).or_insert_with(|| {
				Roast::new(
					self.frost_id,
					self.threshold,
					key_package.clone(),
					public_key_package.clone(),
					self.coordinators.clone(),
				)
			})),
			_ => None,
		}
	}

	pub fn on_start(&mut self, id: I) {
		tracing::info!(peer_id = field::display(self.peer_id.clone()), "start {}", id);
		if self.get_or_insert_session(id).is_none() {
			tracing::error!("not ready to sign");
		}
	}

	pub fn on_sign(&mut self, id: I, data: Vec<u8>) {
		tracing::info!(peer_id = field::display(self.peer_id.clone()), "sign {}", id);
		if let Some(session) = self.get_or_insert_session(id) {
			session.set_data(data)
		} else {
			tracing::error!("not ready to sign");
		}
	}

	pub fn on_complete(&mut self, id: I) {
		tracing::info!(peer_id = field::display(self.peer_id.clone()), "complete {}", id);
		match &mut self.state {
			TssState::Roast { signing_sessions, .. } => {
				signing_sessions.remove(&id);
			},
			_ => {
				tracing::error!("not ready to complete");
			},
		}
	}

	pub fn next_action(&mut self) -> Option<TssAction<I, P>> {
		match &mut self.state {
			TssState::Dkg(dkg) => {
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
					DkgAction::Commit(commitment, proof_of_knowledge) => {
						return Some(TssAction::Commit(commitment, proof_of_knowledge));
					},
					DkgAction::Complete(key_package, public_key_package, commitment) => {
						let signing_share = *key_package.signing_share();
						let public_key =
							VerifyingKey::new(public_key_package.verifying_key().to_element());
						self.state = TssState::Roast {
							key_package,
							public_key_package,
							signing_sessions: Default::default(),
						};
						return Some(TssAction::Ready(signing_share, commitment, public_key));
					},
					DkgAction::Failure(error) => {
						tracing::error!("dkg failed with {:?}", error);
						self.state = TssState::Failed;
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
							RoastAction::Complete(hash, signature) => {
								return Some(TssAction::Signature(id, hash, signature));
							},
						};
						if send_to_self {
							session.on_message(self.frost_id, msg.clone());
						}
						if !peers.is_empty() {
							return Some(TssAction::Send(
								peers
									.iter()
									.map(|peer| {
										(
											self.frost_to_peer(peer),
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
			TssState::Failed => {},
		}
		None
	}
}
