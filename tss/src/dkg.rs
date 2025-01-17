//! # Distributed Key Generation (DKG) Module
//! The DKG module is responsible for managing the Distributed Key Generation
//! process, which allows a set of participants to collectively generate a
//! shared secret key. This process involves multiple rounds of communication
//! and ensures that no single participant has access to the entire secret key.
//! The module supports handling messages, committing to secret shares, and
//! transitioning through different stages of the DKG process.
//!
//!
use frost_evm::frost_secp256k1::Signature;
use frost_evm::keys::dkg::*;
use frost_evm::keys::{
	KeyPackage, PublicKeyPackage, SecretShare, SigningShare, VerifiableSecretSharingCommitment,
};
use frost_evm::{Error, Identifier, Scalar};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// Defines different actions that can occur during the DKG process:
#[derive(Clone)]
pub enum DkgAction {
	/// Send a commitment with a proof.
	Commit(VerifiableSecretSharingCommitment, Signature),
	/// Send messages to other participants.
	Send(Vec<(Identifier, DkgMessage)>),
	/// Completion of the DKG process with a key package and commitment.
	Complete(KeyPackage, PublicKeyPackage, VerifiableSecretSharingCommitment),
	/// Represents a failure in the process.
	Failure(Error),
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub struct DkgMessage(round2::Package);

impl std::fmt::Display for DkgMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "dkg")
	}
}

/// Distributed key generation state machine.
pub struct Dkg {
	id: Identifier,
	members: BTreeSet<Identifier>,
	threshold: u16,
	secret_package: Option<round1::SecretPackage>,
	commitment: Option<VerifiableSecretSharingCommitment>,
	sent_round2_packages: bool,
	round2_packages: HashMap<Identifier, round2::Package>,
}

impl Dkg {
	/// Creates a new instance of the DKG state machine.
	///
	/// ### Arguments
	///
	/// * `id` - The identifier of the current participant.
	/// * `members` - A set of identifiers for all participants.
	/// * `threshold` - The threshold number of participants needed to reconstruct the secret.
	///
	/// ### Returns
	///
	/// A new `Dkg` instance.
	pub fn new(id: Identifier, members: BTreeSet<Identifier>, threshold: u16) -> Self {
		// Ensures the current participant is in the members set.
		debug_assert!(members.contains(&id));
		Self {
			id,
			members,
			threshold,
			secret_package: None,
			commitment: None,
			sent_round2_packages: false,
			round2_packages: Default::default(),
		}
	}

	/// Handles the receipt of a commitment from another participant.
	///
	/// ### Arguments
	///
	/// * `commitment` - The commitment received from another participant.
	pub fn on_commit(&mut self, commitment: VerifiableSecretSharingCommitment) {
		self.commitment = Some(commitment);
	}

	/// Handles the receipt of a message from another participant.
	///
	/// ### Arguments
	///
	/// * `peer` - The identifier of the peer who sent the message.
	/// * `msg` - The message received from the peer.
	pub fn on_message(&mut self, peer: Identifier, msg: DkgMessage) {
		self.round2_packages.insert(peer, msg.0);
	}

	/// Determines the next action to be taken in the DKG process.
	///
	/// ### Returns
	///
	/// An optional `DkgAction` representing the next action to be taken.
	pub fn next_action(&mut self) -> Option<DkgAction> {
		// Check if the secret package is already initialized. If not, perform the first part of the DKG process.
		let Some(secret_package) = self.secret_package.as_ref() else {
			let (secret_package, round1_package) =
				match part1(self.id, self.members.len() as _, self.threshold, OsRng) {
					Ok(result) => result,
					Err(error) => {
						return Some(DkgAction::Failure(error));
					},
				};
			self.secret_package = Some(secret_package);
			return Some(DkgAction::Commit(
				round1_package.commitment().clone(),
				*round1_package.proof_of_knowledge(),
			));
		};
		// Ensure a commitment has been received.
		let commitment = self.commitment.as_ref()?;
		// Prepare to send round 2 packages to peers.
		if !self.sent_round2_packages {
			let mut msgs = Vec::with_capacity(self.members.len());
			for peer in &self.members {
				if *peer == self.id {
					continue;
				}
				let share = SigningShare::from_coefficients(secret_package.coefficients(), *peer);
				msgs.push((*peer, DkgMessage(round2::Package::new(share))));
			}
			self.sent_round2_packages = true;
			return Some(DkgAction::Send(msgs));
		}
		// Check if all round 2 packages have been received.
		if self.round2_packages.len() != self.members.len() - 1 {
			return None;
		}

		// Combine all received signing shares into one signing share.
		let signing_share = self
			.round2_packages
			.values()
			.map(|package| *package.signing_share())
			.chain(std::iter::once(SigningShare::from_coefficients(
				secret_package.coefficients(),
				self.id,
			)))
			.fold(SigningShare::new(Scalar::ZERO), |acc, e| {
				SigningShare::new(acc.to_scalar() + e.to_scalar())
			});
		// Create a secret share and corresponding key package.
		let secret_share = SecretShare::new(self.id, signing_share, commitment.clone());
		let key_package = match KeyPackage::try_from(secret_share) {
			Ok(key_package) => key_package,
			Err(error) => {
				return Some(DkgAction::Failure(error));
			},
		};
		// Create a public key package from the commitment and members.
		let public_key_package =
			PublicKeyPackage::from_commitment(&self.members, commitment).unwrap();
		Some(DkgAction::Complete(key_package, public_key_package, commitment.clone()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frost_evm::frost_core::keys::dkg::verify_proof_of_knowledge;
	use frost_evm::frost_core::keys::{default_identifiers, sum_commitments};

	#[test]
	fn test_dkg() {
		crate::tests::init_logger();
		let members: BTreeSet<_> = default_identifiers(3).into_iter().collect();
		let threshold = 2;
		let mut dkgs: HashMap<_, _> = members
			.iter()
			.map(|id| (*id, Dkg::new(*id, members.clone(), threshold)))
			.collect();
		let mut commitments = Vec::with_capacity(members.len());
		loop {
			for from in &members {
				match dkgs.get_mut(from).unwrap().next_action() {
					Some(DkgAction::Commit(commitment, proof_of_knowledge)) => {
						verify_proof_of_knowledge(*from, &commitment, proof_of_knowledge).unwrap();
						commitments.push(commitment);
						if commitments.len() == members.len() {
							let commitments = commitments.iter().collect::<Vec<_>>();
							let commitment = sum_commitments(&commitments).unwrap();
							for dkg in dkgs.values_mut() {
								dkg.on_commit(commitment.clone());
							}
						}
					},
					Some(DkgAction::Send(msgs)) => {
						for (to, msg) in msgs {
							dkgs.get_mut(&to).unwrap().on_message(*from, msg);
						}
					},
					Some(DkgAction::Complete(_key_package, _public_key_package, _commitment)) => {
						return;
					},
					Some(DkgAction::Failure(_)) => unreachable!(),
					None => {},
				}
			}
		}
	}
}
