use frost_evm::elliptic_curve::PrimeField;
use frost_evm::frost_core::frost::keys::compute_public_key_package;
use frost_evm::frost_secp256k1::Secp256K1Sha256;
use frost_evm::keys::repairable;
use frost_evm::keys::{
	KeyPackage, PublicKeyPackage, SecretShare, VerifiableSecretSharingCommitment,
};
use frost_evm::{Error, Identifier, Scalar};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

struct RtsSession {
	participant: Identifier,
	helpers: BTreeSet<Identifier>,
	deltas: HashMap<Identifier, Scalar>,
	initialized: bool,
}

impl RtsSession {
	pub fn new(participant: Identifier, helpers: BTreeSet<Identifier>) -> Self {
		Self {
			participant,
			helpers,
			deltas: Default::default(),
			initialized: false,
		}
	}

	pub fn on_delta(&mut self, peer: Identifier, delta: Scalar) {
		if self.helpers.contains(&peer) {
			self.deltas.insert(peer, delta);
		}
	}

	pub fn next_action(&mut self, secret_share: &SecretShare) -> Option<RtsAction> {
		if !self.initialized {
			self.initialized = true;
			let helpers: Vec<_> = self.helpers.iter().copied().collect();
			let action = match repairable::repair_share_step_1::<Secp256K1Sha256, _>(
				&helpers,
				secret_share,
				&mut OsRng,
				self.participant,
			) {
				Ok(deltas) => RtsAction::Send(
					deltas
						.into_iter()
						.map(|(identifier, delta)| {
							(
								identifier,
								RtsMessage::Delta {
									participant: self.participant,
									delta: delta.to_bytes().into(),
								},
							)
						})
						.collect(),
				),
				Err(error) => RtsAction::Failure(error),
			};
			return Some(action);
		}
		if self.deltas.len() == self.helpers.len() {
			let deltas: Vec<_> =
				std::mem::take(&mut self.deltas).into_iter().map(|(_, delta)| delta).collect();
			let sigma = repairable::repair_share_step_2(&deltas);
			return Some(RtsAction::Send(vec![(
				self.participant,
				RtsMessage::Sigma { sigma: sigma.to_bytes().into() },
			)]));
		}
		None
	}
}

pub enum RtsAction {
	Send(Vec<(Identifier, RtsMessage)>),
	Complete(KeyPackage, PublicKeyPackage, VerifiableSecretSharingCommitment),
	Failure(Error),
}

#[derive(Clone, Deserialize, Serialize)]
pub enum RtsMessage {
	Init { helpers: BTreeSet<Identifier> },
	Delta { participant: Identifier, delta: [u8; 32] },
	Sigma { sigma: [u8; 32] },
}

impl std::fmt::Display for RtsMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Init { helpers } => write!(f, "init {}", helpers.len()),
			Self::Delta { .. } => write!(f, "delta"),
			Self::Sigma { .. } => write!(f, "sigma"),
		}
	}
}

pub struct RtsHelper {
	secret_share: SecretShare,
	sessions: BTreeMap<Identifier, RtsSession>,
}

impl RtsHelper {
	pub fn new(secret_share: SecretShare) -> Self {
		Self {
			secret_share,
			sessions: Default::default(),
		}
	}

	pub fn on_message(&mut self, peer: Identifier, msg: RtsMessage) {
		match msg {
			RtsMessage::Init { helpers } => {
				self.sessions.insert(peer, RtsSession::new(peer, helpers));
			},
			RtsMessage::Delta { participant, delta } => {
				let Some(session) = self.sessions.get_mut(&participant) else {
					log::error!("TODO: received delta message before init message");
					return;
				};
				if let Some(delta) = Option::from(Scalar::from_repr(delta.into())) {
					session.on_delta(peer, delta);
				}
			},
			msg => log::error!("unexpected message {}", msg),
		}
	}

	pub fn next_action(&mut self) -> Option<RtsAction> {
		for session in self.sessions.values_mut() {
			if let Some(action) = session.next_action(&self.secret_share) {
				return Some(action);
			}
		}
		None
	}
}

pub struct Rts {
	id: Identifier,
	helpers: BTreeSet<Identifier>,
	commitment: VerifiableSecretSharingCommitment,
	public_key_package: PublicKeyPackage,
	initialized: bool,
	sigmas: BTreeMap<Identifier, Scalar>,
}

impl Rts {
	pub fn new(
		id: Identifier,
		commitments: HashMap<Identifier, VerifiableSecretSharingCommitment>,
		threshold: u16,
	) -> Self {
		let public_key_package = compute_public_key_package(&commitments);
		let helpers: BTreeSet<_> = commitments
			.keys()
			.filter(|helper| **helper != id)
			.take(threshold as _)
			.copied()
			.collect();
		let commitment = commitments.get(&id).unwrap().clone();
		Self {
			id,
			commitment,
			public_key_package,
			initialized: false,
			helpers,
			sigmas: Default::default(),
		}
	}

	pub fn on_message(&mut self, peer: Identifier, msg: RtsMessage) {
		match msg {
			RtsMessage::Sigma { sigma } => {
				if let Some(sigma) = Option::from(Scalar::from_repr(sigma.into())) {
					if self.helpers.contains(&peer) {
						self.sigmas.insert(peer, sigma);
					}
				}
			},
			msg => log::error!("unexpected message {}", msg),
		}
	}

	pub fn next_action(&mut self) -> Option<RtsAction> {
		if !self.initialized {
			return Some(RtsAction::Send(
				self.helpers
					.iter()
					.map(|helper| (*helper, RtsMessage::Init { helpers: self.helpers.clone() }))
					.collect(),
			));
		}
		if self.sigmas.len() == self.helpers.len() {
			let sigmas: Vec<_> = self.sigmas.values().copied().collect();
			let secret_share = repairable::repair_share_step_3(&sigmas, self.id, &self.commitment);
			let key_package = match KeyPackage::try_from(secret_share) {
				Ok(key_package) => key_package,
				Err(error) => return Some(RtsAction::Failure(error)),
			};
			return Some(RtsAction::Complete(
				key_package,
				self.public_key_package.clone(),
				self.commitment.clone(),
			));
		}
		None
	}
}
