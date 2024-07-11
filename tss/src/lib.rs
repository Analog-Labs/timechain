#![allow(clippy::large_enum_variant)]
//!
//! # Threshold Signature Scheme (TSS) 
//! The TSS (Threshold Signature Scheme) module handles cryptographic operations related to distributed key generation (DKG) and signature generation using the Roast protocol. This flowchart illustrates the key states and actions within the TSS module.
//! 
#![doc = simple_mermaid::mermaid!("../docs/tss.mmd")]

use crate::dkg::{Dkg, DkgAction, DkgMessage};
use crate::roast::{Roast, RoastAction, RoastMessage};
use anyhow::Result;
use frost_evm::keys::{KeyPackage, PublicKeyPackage, SecretShare};
use frost_evm::Scalar;
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

/// Represents the state of the TSS process.
///
/// - Dkg(Dkg): State during the DKG process.
/// - Roast: State during the ROAST process.
/// - Failed: State when the process has failed.
enum TssState<I> {
	Dkg(Dkg),
	Roast {
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		signing_sessions: BTreeMap<I, Roast>,
	},
	Failed,
}

/// Represents possible actions in the TSS process.
///
/// - 'Send': Action to send messages.
/// - 'Commit': Action to commit a secret.
/// - 'Ready': Action indicating readiness.
/// - 'Signature': Action to provide a signature.
#[derive(Clone)]
pub enum TssAction<I, P> {
	Send(Vec<(P, TssMessage<I>)>),
	Commit(VerifiableSecretSharingCommitment, ProofOfKnowledge),
	Ready(SigningShare, VerifiableSecretSharingCommitment, VerifyingKey),
	Signature(I, [u8; 32], Signature),
}

/// Represents messages in the TSS process.
///
/// - Dkg : Message for DKG.
/// - Roast : Message for ROAST.
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

/// Constructs a proof of knowledge for a given peer, using the provided coefficients and commitment.
///
/// Flow:
/// 1. Converts the peer to a FROST identifier.
/// 2. Uses the FROST library's compute_proof_of_knowledge function to generate the proof of knowledge.
/// 3. Returns the generated proof of knowledge.
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

/// Verifies a proof of knowledge for a given peer, using the provided commitment and proof of knowledge.
///
/// Flow:
/// 1. Converts the peer to a FROST identifier.
/// 2. Uses the FROST library's verify_proof_of_knowledge function to verify the proof of knowledge.
/// 3. Returns the result of the verification.
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
	/// Initializes a new TSS instance with the given parameters.
	///
	/// Flow:
	/// 1. Validates that the peer_id is part of the members.
	/// 2. Converts the peer_id to a FROST identifier.
	/// 3. Maps each member to their FROST identifier.
	/// 4. Selects coordinators from the members.
	/// 5. Determines if the current instance is a coordinator.
	/// 6. If recover is provided, initializes the state with a Roast session; otherwise, initializes with a Dkg session.
	/// 7. Returns the new TSS instance.
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
		tracing::debug!(
			"{} initialize {}/{} coordinator = {}",
			peer_id,
			threshold,
			members.len(),
			is_coordinator
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

	/// Returns the peer ID of the TSS instance.
	pub fn peer_id(&self) -> &P {
		&self.peer_id
	}

	/// Converts a FROST identifier to the corresponding peer ID.
	///
	/// Flow:
	/// 1. Looks up the FROST identifier in the frost_to_peer map.
	/// 2. Returns the corresponding peer ID.
	fn frost_to_peer(&self, frost: &Identifier) -> P {
		self.frost_to_peer.get(frost).unwrap().clone()
	}

	/// Returns the total number of nodes involved in the TSS process.
	pub fn total_nodes(&self) -> usize {
		self.frost_to_peer.len()
	}

	/// Returns the threshold value for the TSS process.
	pub fn threshold(&self) -> usize {
		self.threshold as _
	}

	/// Checks if the TSS instance has committed.
	pub fn committed(&self) -> bool {
		self.committed
	}

	/// Handles incoming messages and updates the state accordingly.
	///
	/// Flow:
	/// 1. Logs the receipt of the message.
	/// 2. Validates that the message is not from the peer itself.
	/// 3. Converts the sender to a FROST identifier and validates it.
	/// 4. Processes the message based on the current state (DKG or ROAST).
	/// 5. Returns the result of the processing.
	pub fn on_message(&mut self, peer_id: P, msg: TssMessage<I>) -> Result<()> {
		tracing::debug!("{} on_message {} {}", self.peer_id, peer_id, msg);
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

	/// Handles commit actions and updates the state accordingly.
	///
	/// Flow
	/// 1. Logs the commit action.
	/// 2. If in the DKG state, processes the commit and sets committed to true.
	/// 3. Logs an error if not in the DKG state.
	pub fn on_commit(&mut self, commitment: VerifiableSecretSharingCommitment) {
		tracing::debug!("{} commit", self.peer_id);
		match &mut self.state {
			TssState::Dkg(dkg) => {
				dkg.on_commit(commitment);
				self.committed = true;
			},
			_ => tracing::error!("unexpected commit"),
		}
	}

	/// Retrieves or inserts a signing session.
	///
	/// Flow:
	/// 1. If in the ROAST state, retrieves or inserts a signing session for the given ID.
	/// 2. Returns the session or None if not in the ROAST state.
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
	/// Starts a signing session for the given ID.
	///
	/// Flow:
	/// 1. Logs the start action.
	/// 2. Inserts a new session if it does not already exist.
	/// 3. Logs an error if not ready to sign.
	pub fn on_start(&mut self, id: I) {
		tracing::info!("{} start {}", self.peer_id, id);
		if self.get_or_insert_session(id).is_none() {
			tracing::error!("not ready to sign");
		}
	}

	/// Signs data for the given session ID.
	///
	/// Flow:
	/// 1. Logs the sign action.
	/// 2. Sets the data for the session if it exists.
	/// 3. Logs an error if not ready to sign.
	pub fn on_sign(&mut self, id: I, data: Vec<u8>) {
		tracing::debug!("{} sign {}", self.peer_id, id);
		if let Some(session) = self.get_or_insert_session(id) {
			session.set_data(data)
		} else {
			tracing::error!("not ready to sign");
		}
	}

	/// Purpose: Completes a signing session for the given ID.
	///
	/// Flow:
	/// 1. Logs the complete action.
	/// 2. Removes the session if it exists.
	/// 3. Logs an error if not ready to sign.
	pub fn on_complete(&mut self, id: I) {
		tracing::debug!("{} complete {}", self.peer_id, id);
		match &mut self.state {
			TssState::Roast { signing_sessions, .. } => {
				signing_sessions.remove(&id);
			},
			_ => {
				tracing::error!("not ready to complete");
			},
		}
	}

	/// Retrieves the next action to be performed by the TSS state machine.
	/// This could be a message to send, a commitment to make, or a signature to produce.
	/// Flow:
	/// 1. If in the DKG state, returns the next DKG action.
	/// 2. If in the ROAST state, returns the next ROAST action.
	/// 3. Returns None if no action is available.
	pub fn next_action(&mut self) -> Option<TssAction<I, P>> {
		match &mut self.state {
			// Handle the DKG state
			TssState::Dkg(dkg) => {
				match dkg.next_action()? {
					// If the next DKG action is to send messages, wrap them in TssAction::Send
					DkgAction::Send(msgs) => {
						return Some(TssAction::Send(
							msgs.into_iter()
								.map(|(peer, msg)| {
									(self.frost_to_peer(&peer), TssMessage::Dkg { msg })
								})
								.collect(),
						));
					},
					// If the next DKG action is to commit, wrap it in TssAction::Commit
					DkgAction::Commit(commitment, proof_of_knowledge) => {
						return Some(TssAction::Commit(commitment, proof_of_knowledge));
					},
					// If the DKG is complete, transition to the ROAST state and prepare to sign
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
					// If the DKG fails, transition to the Failed state
					DkgAction::Failure(error) => {
						tracing::error!("dkg failed with {:?}", error);
						self.state = TssState::Failed;
						return None;
					},
				};
			},
			// Handle the ROAST state
			TssState::Roast { signing_sessions, .. } => {
				let session_ids: Vec<_> = signing_sessions.keys().cloned().collect();
				for id in session_ids {
					let session = signing_sessions.get_mut(&id).unwrap();
					while let Some(action) = session.next_action() {
						let (peers, send_to_self, msg) = match action {
							// If the next ROAST action is to send a message to a peer
							RoastAction::Send(peer, msg) => {
								if peer == self.frost_id {
									(vec![], true, msg)
								} else {
									(vec![peer], false, msg)
								}
							},
							// If the next ROAST action is to send messages to multiple peers
							RoastAction::SendMany(all_peers, msg) => {
								let peers: Vec<_> = all_peers
									.iter()
									.filter(|peer| **peer != self.frost_id)
									.copied()
									.collect();
								let send_to_self = peers.len() != all_peers.len();
								(peers, send_to_self, msg)
							},
							// If the ROAST action is complete, produce a signature
							RoastAction::Complete(hash, signature) => {
								return Some(TssAction::Signature(id, hash, signature));
							},
						};
						// Handle sending the message to self if needed
						if send_to_self {
							session.on_message(self.frost_id, msg.clone());
						}
						// Handle sending the message to peers if needed
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
