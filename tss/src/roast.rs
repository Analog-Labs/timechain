//! # Roast Module Documentation
//!
//! The Roast module implements a state machine for managing the Robust Online
//! Asynchronous Schnorr Threshold (ROAST) protocol. This protocol allows a
//! distributed set of signers to collaboratively generate Schnorr signatures
//! in a secure and robust manner, ensuring that the signature process can
//! continue even if some participants fail to respond or act maliciously.
//!
//! ## Overview
//! The main components of the Roast module include:
//!
//! - 'RoastSigner': Manages the signing process for a single participant.
//! - 'RoastCoordinator': Coordinates the signing process, ensuring that enough participants have committed to generate a signature.
//! - 'RoastSession': Manages a single signing session, tracking commitments and signature shares.
//! - 'Roast': The main state machine that brings together the RoastSigner and RoastCoordinator to manage the overall ROAST protocol.
//!
use frost_evm::{
	keys::{KeyPackage, PublicKeyPackage},
	round1::{self, SigningCommitments, SigningNonces},
	round2::{self, SignatureShare},
	Identifier, Signature, SigningPackage, VerifyingKey,
};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use tracing::{Level, Span};

/// Represents a signing request sent to a RoastSigner.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RoastSignerRequest {
	session_id: u16,
	commitments: BTreeMap<Identifier, SigningCommitments>,
}

/// Represents a response from a RoastSigner containing a signature share and a new commitment.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RoastSignerResponse {
	session_id: u16,
	signature_share: SignatureShare,
	commitment: SigningCommitments,
}

struct RoastSigner {
	key_package: KeyPackage,
	data: Option<Vec<u8>>,
	coordinators: BTreeMap<Identifier, SigningNonces>,
	requests: VecDeque<(Identifier, RoastSignerRequest)>,
	span: Span,
}

impl RoastSigner {
	/// Creates a new `RoastSigner` instance with the given key package.
	pub fn new(key_package: KeyPackage, span: &Span) -> Self {
		let span = tracing::span!(parent: span, Level::INFO, "signer");
		Self {
			key_package,
			data: None,
			coordinators: Default::default(),
			requests: Default::default(),
			span,
		}
	}

	/// Sets the data to be signed.
	pub fn set_data(&mut self, data: Vec<u8>) {
		self.data = Some(data);
	}

	/// Returns the data to be signed, if any.
	pub fn data(&self) -> Option<&[u8]> {
		self.data.as_deref()
	}

	/// Commits to the signing process for a coordinator.
	pub fn commit(&mut self, coordinator: Identifier) -> SigningCommitments {
		let (nonces, commitment) = round1::commit(self.key_package.signing_share(), &mut OsRng);
		self.coordinators.insert(coordinator, nonces);
		commitment
	}

	/// Handles a signing request from a coordinator.
	pub fn sign(&mut self, coordinator: Identifier, request: RoastSignerRequest) {
		self.requests.push_back((coordinator, request));
	}

	/// Generates a message with the signature share for a coordinator.
	pub fn message(&mut self) -> Option<(Identifier, RoastSignerResponse)> {
		let data = self.data.as_deref()?;
		loop {
			let (coordinator, request) = self.requests.pop_front()?;
			let session_id = request.session_id;
			let signing_package = SigningPackage::new(request.commitments, data);
			let nonces = self
				.coordinators
				.remove(&coordinator)
				.expect("we sent the coordinator a commitment");
			let signature_share = match round2::sign(&signing_package, &nonces, &self.key_package) {
				Ok(ss) => ss,
				Err(err) => {
					tracing::error!(parent: &self.span, session_id = session_id, "invalid signing package {err:?}");
					continue;
				},
			};
			let commitment = self.commit(coordinator);
			return Some((
				coordinator,
				RoastSignerResponse {
					session_id,
					signature_share,
					commitment,
				},
			));
		}
	}
}

/// Manages a single signing session, tracking commitments and signature shares.
struct RoastSession {
	commitments: BTreeMap<Identifier, SigningCommitments>,
	signature_shares: HashMap<Identifier, SignatureShare>,
}

impl RoastSession {
	/// Creates a new `RoastSession` instance with the given commitments.
	fn new(commitments: BTreeMap<Identifier, SigningCommitments>) -> Self {
		Self {
			commitments,
			signature_shares: Default::default(),
		}
	}

	/// Handles a signature share from a peer.
	fn on_signature_share(&mut self, peer: Identifier, signature_share: SignatureShare) {
		if self.commitments.contains_key(&peer) {
			self.signature_shares.insert(peer, signature_share);
		}
	}

	/// Checks if the session is complete, i.e., if all required signature shares have been received.
	fn is_complete(&self) -> bool {
		self.commitments.len() == self.signature_shares.len()
	}
}

/// Coordinates the signing process, ensuring that enough participants have committed to generate a signature.
struct RoastCoordinator {
	threshold: u16,
	session_id: u16,
	commitments: BTreeMap<Identifier, SigningCommitments>,
	sessions: BTreeMap<u16, RoastSession>,
	committed: BTreeSet<Identifier>,
	span: Span,
}

impl RoastCoordinator {
	/// Creates a new `RoastCoordinator` instance with the given threshold.
	fn new(threshold: u16, span: &Span) -> Self {
		let span = tracing::span!(parent: span, Level::INFO, "coordinator");
		Self {
			threshold,
			session_id: 0,
			commitments: Default::default(),
			sessions: Default::default(),
			committed: Default::default(),
			span,
		}
	}

	/// Handles a commitment from a peer.
	fn on_commit(&mut self, peer: Identifier, commitment: SigningCommitments) {
		if !self.committed.contains(&peer) {
			self.commitments.insert(peer, commitment);
			self.committed.insert(peer);
		}
	}

	/// Handles a response from a peer.
	fn on_response(&mut self, peer: Identifier, message: RoastSignerResponse) {
		if let Some(session) = self.sessions.get_mut(&message.session_id) {
			self.commitments.insert(peer, message.commitment);
			session.on_signature_share(peer, message.signature_share);
		}
	}

	/// Starts a new signing session if enough commitments have been received.
	fn start_session(&mut self) -> Option<RoastSignerRequest> {
		if self.commitments.len() < self.threshold as _ {
			tracing::debug!(
				parent: &self.span,
				session_id = self.session_id,
				"commitments {}/{}",
				self.commitments.len(),
				self.threshold
			);
			return None;
		}
		let session_id = self.session_id;
		self.session_id = session_id.wrapping_add(1);
		let mut commitments = std::mem::take(&mut self.commitments);
		while commitments.len() > self.threshold as _ {
			let (peer, commitment) = commitments.pop_last().unwrap();
			self.commitments.insert(peer, commitment);
		}
		self.sessions.insert(session_id, RoastSession::new(commitments.clone()));
		Some(RoastSignerRequest { session_id, commitments })
	}

	/// Aggregates the signature shares from a complete session.
	fn aggregate_signature(&mut self) -> Option<RoastSession> {
		let session_id = self
			.sessions
			.iter()
			.filter(|(_, session)| session.is_complete())
			.map(|(session_id, _)| *session_id)
			.next()?;
		self.sessions.remove(&session_id)
	}
}

/// Represents a message exchanged between Roast participants.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum RoastMessage {
	Commit(SigningCommitments),
	Sign(RoastSignerRequest),
	Signature(RoastSignerResponse),
}

impl RoastMessage {
	/// Checks if the message is a response.
	pub fn is_response(&self) -> bool {
		matches!(self, Self::Signature(_))
	}
}

impl std::fmt::Display for RoastMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Commit(_) => write!(f, "commit"),
			Self::Sign(_) => write!(f, "sign"),
			Self::Signature(_) => write!(f, "signature"),
		}
	}
}

/// Represents an action to be taken by the Roast state machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoastAction {
	Send(Identifier, RoastMessage),
	SendMany(Vec<Identifier>, RoastMessage),
	Complete([u8; 32], Signature),
}

/// The main state machine that manages the overall ROAST protocol by integrating ['RoastSigner'] and ['RoastCoordinator'].
pub struct Roast {
	signer: RoastSigner,
	coordinator: Option<RoastCoordinator>,
	public_key_package: PublicKeyPackage,
	coordinators: BTreeSet<Identifier>,
	_span: Span,
}

impl Roast {
	/// Creates a new `Roast` instance with the given parameters.
	pub fn new(
		id: Identifier,
		threshold: u16,
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		coordinators: BTreeSet<Identifier>,
		span: &Span,
	) -> Self {
		let span = tracing::span!(parent: span, Level::INFO, "roast");
		let is_coordinator = coordinators.contains(&id);
		Self {
			signer: RoastSigner::new(key_package, &span),
			coordinator: if is_coordinator {
				Some(RoastCoordinator::new(threshold, &span))
			} else {
				None
			},
			public_key_package,
			coordinators,
			_span: span,
		}
	}

	/// Sets the data to be signed.
	pub fn set_data(&mut self, data: Vec<u8>) {
		self.signer.set_data(data);
	}

	/// Handles an incoming message from a peer.
	pub fn on_message(&mut self, peer: Identifier, msg: RoastMessage) {
		match msg {
			RoastMessage::Commit(commitment) => {
				if let Some(coordinator) = self.coordinator.as_mut() {
					coordinator.on_commit(peer, commitment);
				}
			},
			RoastMessage::Sign(request) => {
				self.signer.sign(peer, request);
			},
			RoastMessage::Signature(response) => {
				if let Some(coordinator) = self.coordinator.as_mut() {
					coordinator.on_response(peer, response);
				}
			},
		}
	}

	/// Determines the next action to be taken by the state machine.
	pub fn next_action(&mut self) -> Option<RoastAction> {
		if let Some(coordinator) = self.coordinator.as_mut() {
			if let Some(data) = self.signer.data() {
				if let Some(session) = coordinator.aggregate_signature() {
					let signing_package = SigningPackage::new(session.commitments, data);
					if let Ok(signature) = frost_evm::aggregate(
						&signing_package,
						&session.signature_shares,
						&self.public_key_package,
					) {
						let hash = VerifyingKey::message_hash(data);
						self.coordinator.take();
						return Some(RoastAction::Complete(hash, signature));
					}
				}
			}
			if let Some(request) = coordinator.start_session() {
				let peers = request.commitments.keys().copied().collect();
				return Some(RoastAction::SendMany(peers, RoastMessage::Sign(request)));
			}
		}
		if let Some(coordinator) = self.coordinators.pop_last() {
			return Some(RoastAction::Send(
				coordinator,
				RoastMessage::Commit(self.signer.commit(coordinator)),
			));
		}
		if let Some((coordinator, response)) = self.signer.message() {
			return Some(RoastAction::Send(coordinator, RoastMessage::Signature(response)));
		}
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use frost_evm::keys::{generate_with_dealer, IdentifierList};
	use tracing::Level;

	/// The module includes a test for the Roast protocol to ensure its correctness.
	#[test]
	fn test_roast() -> Result<()> {
		crate::tests::init_logger();
		let signers = 3;
		let threshold = 2;
		let coordinator = 1;
		let data = [1u8; 32];
		let (secret_shares, public_key_package) =
			generate_with_dealer(signers, threshold, IdentifierList::Default, OsRng).unwrap();
		let coordinators: BTreeSet<_> = secret_shares.keys().copied().take(coordinator).collect();
		let mut roasts: BTreeMap<_, _> = secret_shares
			.into_iter()
			.map(|(peer, secret_share)| {
				(
					peer,
					Roast::new(
						peer,
						threshold,
						KeyPackage::try_from(secret_share).unwrap(),
						public_key_package.clone(),
						coordinators.clone(),
						&tracing::span!(Level::INFO, "shard"),
					),
				)
			})
			.collect();
		let members: Vec<_> = roasts.keys().copied().collect();
		for roast in roasts.values_mut() {
			roast.set_data(data.into());
		}
		loop {
			for from in &members {
				if let Some(action) = roasts.get_mut(from).unwrap().next_action() {
					match action {
						RoastAction::Send(to, commitment) => {
							roasts.get_mut(&to).unwrap().on_message(*from, commitment);
						},
						RoastAction::SendMany(peers, request) => {
							for to in peers {
								roasts.get_mut(&to).unwrap().on_message(*from, request.clone());
							}
						},
						RoastAction::Complete(_hash, _signature) => {
							return Ok(());
						},
					}
				}
			}
		}
	}
}
