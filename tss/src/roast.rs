use frost_evm::{
	keys::{KeyPackage, PublicKeyPackage},
	round1::{self, SigningCommitments, SigningNonces},
	round2::{self, SignatureShare},
	Error, Identifier, Signature, SigningPackage, VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

enum RoastState {
	Uninitialized,
	Commit {
		nonces: SigningNonces,
		commitments: BTreeMap<Identifier, SigningCommitments>,
		signature_shares: HashMap<Identifier, SignatureShare>,
	},
	Sign {
		signing_package: SigningPackage,
		signature_shares: HashMap<Identifier, SignatureShare>,
	},
}

impl std::fmt::Display for RoastState {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Uninitialized => write!(f, "uninitialized"),
			Self::Commit { commitments, .. } => write!(f, "commit {}", commitments.len()),
			Self::Sign { signature_shares, .. } => write!(f, "sign {}", signature_shares.len()),
		}
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub enum RoastMessage {
	Commit { commitment: SigningCommitments },
	Sign { signature_share: SignatureShare },
}

impl std::fmt::Display for RoastMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Commit { .. } => write!(f, "commit"),
			Self::Sign { .. } => write!(f, "sign"),
		}
	}
}

pub enum RoastAction {
	Broadcast(RoastMessage),
	Complete([u8; 32], Signature),
	Failure(Option<Identifier>, Error),
}

/// ROAST state machine.
pub struct Roast {
	id: Identifier,
	members: BTreeSet<Identifier>,
	key_package: KeyPackage,
	public_key_package: PublicKeyPackage,
	data: Vec<u8>,
	state: RoastState,
}

impl Roast {
	pub fn new(
		id: Identifier,
		members: BTreeSet<Identifier>,
		_threshold: u16,
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		data: Vec<u8>,
	) -> Self {
		debug_assert!(members.contains(&id));
		Self {
			id,
			members,
			key_package,
			public_key_package,
			data,
			state: RoastState::Uninitialized,
		}
	}

	pub fn on_message(&mut self, peer: Identifier, msg: RoastMessage) {
		match (&mut self.state, msg) {
			(RoastState::Uninitialized, _) => log::error!("received message before polling state"),
			(RoastState::Commit { commitments, .. }, RoastMessage::Commit { commitment }) => {
				commitments.insert(peer, commitment);
			},
			(
				RoastState::Commit { signature_shares, .. },
				RoastMessage::Sign { signature_share },
			) => {
				signature_shares.insert(peer, signature_share);
			},
			(RoastState::Sign { signature_shares, .. }, RoastMessage::Sign { signature_share }) => {
				signature_shares.insert(peer, signature_share);
			},
			(RoastState::Sign { .. }, RoastMessage::Commit { .. }) => {
				log::error!("received commit message in signing state");
			},
		}
	}

	pub fn next_action(&mut self) -> Option<RoastAction> {
		match &mut self.state {
			RoastState::Uninitialized => {
				let (nonces, commitment) =
					round1::commit(self.key_package.secret_share(), &mut OsRng);
				self.state = RoastState::Commit {
					nonces,
					commitments: {
						let mut commitments = BTreeMap::new();
						commitments.insert(self.id, commitment);
						commitments
					},
					signature_shares: Default::default(),
				};
				return Some(RoastAction::Broadcast(RoastMessage::Commit { commitment }));
			},
			RoastState::Commit {
				nonces,
				commitments,
				signature_shares,
			} => {
				if commitments.len() == self.members.len() {
					log::debug!("received all commitments processing signing");
					let commitments = std::mem::take(commitments);
					let signing_package = SigningPackage::new(commitments, &self.data);
					let signature_share = round2::sign(&signing_package, nonces, &self.key_package)
						.expect("valid inputs");
					let mut signature_shares = std::mem::take(signature_shares);
					signature_shares.insert(self.id, signature_share);
					self.state = RoastState::Sign {
						signing_package,
						signature_shares,
					};
					return Some(RoastAction::Broadcast(RoastMessage::Sign { signature_share }));
				}
			},
			RoastState::Sign {
				signing_package,
				signature_shares,
			} => {
				if signature_shares.len() == self.members.len() {
					log::debug!("Received all shares processing aggregator");
					let shares = std::mem::take(signature_shares);
					let hash = VerifyingKey::message_hash(signing_package.message());
					match frost_evm::aggregate(signing_package, &shares, &self.public_key_package) {
						Ok(signature) => {
							return Some(RoastAction::Complete(hash, signature));
						},
						Err(Error::InvalidSignatureShare { culprit }) => {
							return Some(RoastAction::Failure(
								Some(culprit),
								Error::InvalidSignatureShare { culprit },
							));
						},
						Err(error) => {
							return Some(RoastAction::Failure(None, error));
						},
					}
				}
			},
		};
		None
	}
}

impl std::fmt::Display for Roast {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		self.state.fmt(f)
	}
}
