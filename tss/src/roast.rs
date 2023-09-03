use frost_evm::{
	keys::{KeyPackage, PublicKeyPackage},
	round1::{self, SigningCommitments, SigningNonces},
	round2::{self, SignatureShare},
	Identifier, Signature, SigningPackage, VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Default)]
struct Commitments {
	commitments: BTreeMap<Identifier, SigningCommitments>,
}

impl Commitments {
	fn insert(&mut self, peer: Identifier, commitment: SigningCommitments) {
		self.commitments.insert(peer, commitment);
	}

	fn get(
		&self,
		members: &BTreeSet<Identifier>,
	) -> Option<BTreeMap<Identifier, SigningCommitments>> {
		let commitments: BTreeMap<_, _> = self
			.commitments
			.iter()
			.filter(|(id, _)| members.contains(id))
			.map(|(id, commitment)| (*id, *commitment))
			.collect();
		if commitments.len() == members.len() {
			Some(commitments)
		} else {
			None
		}
	}
}

#[derive(Default)]
struct RoastSigner {
	sessions: BTreeMap<(Identifier, u16), BTreeSet<Identifier>>,
}

impl RoastSigner {
	fn on_start_session(
		&mut self,
		peer: Identifier,
		session_id: u16,
		members: BTreeSet<Identifier>,
	) {
		self.sessions.insert((peer, session_id), members);
	}

	fn send_signing_share(
		&mut self,
		commitments: &Commitments,
	) -> Option<(Identifier, u16, BTreeMap<Identifier, SigningCommitments>)> {
		for (coordinator, session_id) in self.sessions.keys().copied() {
			let members = self.sessions.get(&(coordinator, session_id))?;
			if let Some(commitments) = commitments.get(members) {
				self.sessions.remove(&(coordinator, session_id));
				return Some((coordinator, session_id, commitments));
			}
		}
		None
	}
}

struct RoastSession {
	commitments: BTreeMap<Identifier, SigningCommitments>,
	signature_shares: HashMap<Identifier, SignatureShare>,
}

impl RoastSession {
	fn new(commitments: BTreeMap<Identifier, SigningCommitments>) -> Self {
		Self {
			commitments,
			signature_shares: Default::default(),
		}
	}

	fn on_signature_share(&mut self, peer: Identifier, signature_share: SignatureShare) {
		if self.commitments.contains_key(&peer) {
			self.signature_shares.insert(peer, signature_share);
		}
	}

	fn is_complete(&self) -> bool {
		self.commitments.len() == self.signature_shares.len()
	}
}

struct RoastCoordinator {
	threshold: u16,
	session_id: u16,
	responsive: BTreeSet<Identifier>,
	sessions: BTreeMap<u16, RoastSession>,
}

impl RoastCoordinator {
	fn new(threshold: u16) -> Self {
		Self {
			threshold,
			session_id: 0,
			responsive: Default::default(),
			sessions: Default::default(),
		}
	}

	fn on_commit(&mut self, peer: Identifier) {
		log::debug!("marking responsive {:?}", peer);
		self.responsive.insert(peer);
	}

	fn on_sign(&mut self, session_id: u16, peer: Identifier, signature_share: SignatureShare) {
		if let Some(session) = self.sessions.get_mut(&session_id) {
			self.responsive.insert(peer);
			session.on_signature_share(peer, signature_share);
		}
	}

	fn start_session(&mut self, commitments: &Commitments) -> Option<(u16, BTreeSet<Identifier>)> {
		if self.responsive.len() < self.threshold as _ {
			log::debug!("responsive {}/{}", self.responsive.len(), self.threshold);
			return None;
		}
		let session_id = self.session_id;
		self.session_id += 1;
		let mut members = std::mem::take(&mut self.responsive);
		while members.len() > self.threshold as _ {
			self.responsive.insert(members.pop_last().unwrap());
		}
		let commitments = commitments.get(&members).unwrap();
		self.sessions.insert(session_id, RoastSession::new(commitments));
		Some((session_id, members))
	}

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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum RoastMessage {
	Commit { commitment: SigningCommitments },
	StartSession { session_id: u16, members: BTreeSet<Identifier> },
	Sign { session_id: u16, signature_share: SignatureShare },
}

impl std::fmt::Display for RoastMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Commit { .. } => write!(f, "commit"),
			Self::StartSession { .. } => write!(f, "start session"),
			Self::Sign { .. } => write!(f, "sign"),
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoastAction {
	Broadcast(RoastMessage),
	Send(Identifier, RoastMessage),
	SendMany(Vec<Identifier>, RoastMessage),
	Complete([u8; 32], Signature),
}

/// ROAST state machine.
pub struct Roast {
	key_package: KeyPackage,
	public_key_package: PublicKeyPackage,
	data: Vec<u8>,
	nonces: Option<SigningNonces>,
	commitments: Commitments,
	signer: RoastSigner,
	coordinator: Option<RoastCoordinator>,
}

impl Roast {
	pub fn new(
		threshold: u16,
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		data: Vec<u8>,
		is_coordinator: bool,
	) -> Self {
		Self {
			key_package,
			public_key_package,
			data,
			nonces: None,
			commitments: Default::default(),
			signer: RoastSigner::default(),
			coordinator: if is_coordinator { Some(RoastCoordinator::new(threshold)) } else { None },
		}
	}

	pub fn on_message(&mut self, peer: Identifier, msg: RoastMessage) {
		match msg {
			RoastMessage::Commit { commitment } => {
				self.commitments.insert(peer, commitment);
				if let Some(coordinator) = self.coordinator.as_mut() {
					coordinator.on_commit(peer);
				}
			},
			RoastMessage::StartSession { session_id, members } => {
				self.signer.on_start_session(peer, session_id, members);
			},
			RoastMessage::Sign { session_id, signature_share } => {
				if let Some(coordinator) = self.coordinator.as_mut() {
					coordinator.on_sign(session_id, peer, signature_share);
				}
			},
		}
	}

	pub fn next_action(&mut self) -> Option<RoastAction> {
		let Some(nonces) = self.nonces.as_ref() else {
			let (nonces, commitment) = round1::commit(self.key_package.secret_share(), &mut OsRng);
			self.nonces = Some(nonces);
			return Some(RoastAction::Broadcast(RoastMessage::Commit { commitment }));
		};
		if let Some((coordinator, session_id, commitments)) =
			self.signer.send_signing_share(&self.commitments)
		{
			let signing_package = SigningPackage::new(commitments, &self.data);
			if let Ok(signature_share) = round2::sign(&signing_package, nonces, &self.key_package) {
				return Some(RoastAction::Send(
					coordinator,
					RoastMessage::Sign { session_id, signature_share },
				));
			}
		}
		if let Some(coordinator) = self.coordinator.as_mut() {
			if let Some(session) = coordinator.aggregate_signature() {
				let signing_package = SigningPackage::new(session.commitments, &self.data);
				if let Ok(signature) = frost_evm::aggregate(
					&signing_package,
					&session.signature_shares,
					&self.public_key_package,
				) {
					let hash = VerifyingKey::message_hash(&self.data);
					return Some(RoastAction::Complete(hash, signature));
				}
			}
			if let Some((session_id, members)) = coordinator.start_session(&self.commitments) {
				return Some(RoastAction::SendMany(
					members.iter().copied().collect(),
					RoastMessage::StartSession { session_id, members },
				));
			}
		}
		None
	}
}
