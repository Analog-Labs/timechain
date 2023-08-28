use frost_evm::frost_secp256k1::Signature;
use frost_evm::keys::dkg::*;
use frost_evm::keys::{KeyPackage, PublicKeyPackage, VerifiableSecretSharingCommitment};
use frost_evm::{Error, Identifier};

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Dkg state.
enum DkgState {
	Uninitialized,
	DkgR1 {
		secret_package: round1::SecretPackage,
		proofs_of_knowledge: HashMap<Identifier, Signature>,
		commitments: Option<HashMap<Identifier, VerifiableSecretSharingCommitment>>,
		round2_packages: HashMap<Identifier, round2::Package>,
	},
	DkgR2 {
		secret_package: round2::SecretPackage,
		round1_packages: HashMap<Identifier, round1::Package>,
		round2_packages: HashMap<Identifier, round2::Package>,
	},
}

impl std::fmt::Display for DkgState {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Uninitialized => write!(f, "uninitialized"),
			Self::DkgR1 { proofs_of_knowledge, .. } => {
				write!(f, "dkgr1 {}", proofs_of_knowledge.len())
			},
			Self::DkgR2 { round2_packages, .. } => write!(f, "dkgr2 {}", round2_packages.len()),
		}
	}
}

#[derive(Clone)]
pub enum DkgAction {
	Broadcast(DkgMessage),
	Send(Vec<(Identifier, DkgMessage)>),
	Commit(VerifiableSecretSharingCommitment),
	Complete(KeyPackage, PublicKeyPackage, VerifiableSecretSharingCommitment),
	Failure(Error),
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub enum DkgMessage {
	DkgR1 { proof_of_knowledge: Signature },
	DkgR2 { round2_package: round2::Package },
}

impl std::fmt::Display for DkgMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::DkgR1 { .. } => write!(f, "dkgr1"),
			Self::DkgR2 { .. } => write!(f, "dkgr2"),
		}
	}
}

/// Distributed key generation state machine.
pub struct Dkg {
	id: Identifier,
	members: BTreeSet<Identifier>,
	threshold: u16,
	state: DkgState,
	commitment: Option<VerifiableSecretSharingCommitment>,
	committed: bool,
}

impl Dkg {
	pub fn new(id: Identifier, members: BTreeSet<Identifier>, threshold: u16) -> Self {
		debug_assert!(members.contains(&id));
		Self {
			id,
			members,
			threshold,
			state: DkgState::Uninitialized,
			commitment: None,
			committed: false,
		}
	}

	pub fn commit(
		&mut self,
		secure_broadcast_commitments: HashMap<Identifier, VerifiableSecretSharingCommitment>,
	) {
		match &mut self.state {
			DkgState::DkgR1 { commitments, .. } => {
				if commitments.is_none() {
					*commitments = Some(secure_broadcast_commitments);
				} else {
					log::error!("already received commitments");
				}
			},
			state => log::error!("unexpected commitments in state {}", state),
		}
	}

	pub fn on_message(&mut self, peer: Identifier, msg: DkgMessage) {
		match (&mut self.state, msg) {
			(DkgState::Uninitialized, _) => log::error!("received msg before polling state"),
			(
				DkgState::DkgR1 { proofs_of_knowledge, .. },
				DkgMessage::DkgR1 { proof_of_knowledge, .. },
			) => {
				proofs_of_knowledge.insert(peer, proof_of_knowledge);
			},
			(DkgState::DkgR1 { round2_packages, .. }, DkgMessage::DkgR2 { round2_package, .. }) => {
				round2_packages.insert(peer, round2_package);
			},
			(DkgState::DkgR2 { round2_packages, .. }, DkgMessage::DkgR2 { round2_package, .. }) => {
				round2_packages.insert(peer, round2_package);
			},
			(DkgState::DkgR2 { .. }, DkgMessage::DkgR1 { .. }) => {
				log::error!("received dkgr1 message in round2");
			},
		}
	}

	pub fn next_action(&mut self) -> Option<DkgAction> {
		if !self.committed {
			if let Some(commitment) = self.commitment.as_ref() {
				self.committed = true;
				return Some(DkgAction::Commit(commitment.clone()));
			}
		}
		match &mut self.state {
			DkgState::Uninitialized => {
				let (secret_package, round1_package) =
					part1(self.id, self.members.len() as _, self.threshold, OsRng).unwrap();
				self.state = DkgState::DkgR1 {
					secret_package,
					proofs_of_knowledge: Default::default(),
					commitments: Default::default(),
					round2_packages: Default::default(),
				};
				self.commitment = Some(round1_package.commitment().clone());
				return Some(DkgAction::Broadcast(DkgMessage::DkgR1 {
					proof_of_knowledge: *round1_package.proof_of_knowledge(),
				}));
			},
			DkgState::DkgR1 {
				secret_package,
				proofs_of_knowledge,
				commitments,
				round2_packages,
			} => {
				if commitments.is_some() && proofs_of_knowledge.len() == self.members.len() - 1 {
					log::debug!("received all packages for dk2 processing transition");
					let secret_package = secret_package.clone();
					let proofs_of_knowledge = std::mem::take(proofs_of_knowledge);
					let mut commitments = commitments.take().unwrap();
					let round1_packages = proofs_of_knowledge
						.into_iter()
						.map(|(peer, proof_of_knowledge)| {
							(
								peer,
								round1::Package::new(
									commitments.remove(&peer).unwrap(),
									proof_of_knowledge,
								),
							)
						})
						.collect();
					match part2(secret_package, &round1_packages) {
						Ok((secret_package, round2_package)) => {
							self.state = DkgState::DkgR2 {
								secret_package,
								round1_packages,
								round2_packages: std::mem::take(round2_packages),
							};
							let messages = round2_package
								.into_iter()
								.map(|(identifier, round2_package)| {
									(identifier, DkgMessage::DkgR2 { round2_package })
								})
								.collect();
							return Some(DkgAction::Send(messages));
						},
						Err(err) => {
							return Some(DkgAction::Failure(err));
						},
					}
				}
			},
			DkgState::DkgR2 {
				secret_package,
				round1_packages,
				round2_packages,
			} => {
				if round2_packages.len() == self.members.len() - 1 {
					log::debug!("received all packages for dk3 processing transition");
					let round2_packages = std::mem::take(round2_packages);
					let commitment = self.commitment.take().unwrap();
					match part3(secret_package, round1_packages, &round2_packages) {
						Ok((key_package, public_key_package)) => {
							return Some(DkgAction::Complete(
								key_package,
								public_key_package,
								commitment,
							));
						},
						Err(err) => {
							return Some(DkgAction::Failure(err));
						},
					}
				}
			},
		}
		None
	}
}
