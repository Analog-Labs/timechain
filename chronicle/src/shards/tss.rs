use crate::network::PeerId;
use anyhow::Result;
use std::collections::BTreeSet;
pub use time_primitives::TssId;
pub use tss::{SigningKey, VerifiableSecretSharingCommitment, VerifyingKey};

pub type TssMessage = tss::TssMessage<TssId>;
pub type TssAction = tss::TssAction<TssId, PeerId>;

pub enum Tss {
	Enabled(tss::Tss<TssId, String>),
	Disabled(SigningKey, Option<tss::TssAction<TssId, String>>, bool),
}

impl Tss {
	pub fn new(
		peer_id: PeerId,
		members: BTreeSet<PeerId>,
		threshold: u16,
		commitment: Option<VerifiableSecretSharingCommitment>,
	) -> Result<Self> {
		let peer_id = peernet::PeerId::from_bytes(&peer_id)?.to_string();
		let members: Result<BTreeSet<String>> =
			members.into_iter().try_fold(BTreeSet::new(), |mut acc, peer| {
				let peer_id_str = peernet::PeerId::from_bytes(&peer)?.to_string();
				acc.insert(peer_id_str);
				Ok(acc)
			});
		let members = members?;
		if members.len() == 1 {
			let key = SigningKey::random();
			let public = key.public().to_bytes().unwrap();
			let commitment = VerifiableSecretSharingCommitment::deserialize(vec![public]).unwrap();
			let proof_of_knowledge = tss::construct_proof_of_knowledge(
				peer_id,
				&[*key.to_scalar().as_ref()],
				&commitment,
			)
			.unwrap();
			Ok(Tss::Disabled(
				key,
				Some(tss::TssAction::Commit(commitment, proof_of_knowledge)),
				false,
			))
		} else {
			Ok(Tss::Enabled(tss::Tss::new(peer_id, members, threshold, commitment)))
		}
	}

	pub fn committed(&self) -> bool {
		match self {
			Self::Enabled(tss) => tss.committed(),
			Self::Disabled(_, _, committed) => *committed,
		}
	}

	pub fn on_commit(&mut self, commitment: VerifiableSecretSharingCommitment) {
		match self {
			Self::Enabled(tss) => tss.on_commit(commitment),
			Self::Disabled(key, actions, committed) => {
				*actions = Some(tss::TssAction::PublicKey(key.public()));
				*committed = true;
			},
		}
	}

	pub fn on_sign(&mut self, request_id: TssId, data: Vec<u8>) {
		match self {
			Self::Enabled(tss) => tss.on_sign(request_id, data),
			Self::Disabled(key, actions, _) => {
				let hash = VerifyingKey::message_hash(&data);
				*actions =
					Some(tss::TssAction::Signature(request_id, hash, key.sign_prehashed(hash)));
			},
		}
	}

	pub fn on_complete(&mut self, request_id: TssId) {
		match self {
			Self::Enabled(tss) => tss.on_complete(request_id),
			Self::Disabled(_, _, _) => {},
		}
	}

	pub fn on_message(&mut self, peer_id: PeerId, msg: TssMessage) -> Result<Option<TssMessage>> {
		let peer_id = peernet::PeerId::from_bytes(&peer_id)?.to_string();
		Ok(match self {
			Self::Enabled(tss) => tss.on_message(peer_id, msg),
			Self::Disabled(_, _, _) => None,
		})
	}

	pub fn next_action(&mut self) -> Option<TssAction> {
		let action = match self {
			Self::Enabled(tss) => tss.next_action(),
			Self::Disabled(_, action, _) => action.take(),
		}?;
		Some(match action {
			tss::TssAction::Send(msgs) => TssAction::Send(
				msgs.into_iter()
					.map(|(peer, msg)| {
						let peer: peernet::PeerId = peer.parse().unwrap();
						(*peer.as_bytes(), msg)
					})
					.collect(),
			),
			tss::TssAction::Commit(commitment, proof_of_knowledge) => {
				TssAction::Commit(commitment, proof_of_knowledge)
			},
			tss::TssAction::PublicKey(public_key) => TssAction::PublicKey(public_key),
			tss::TssAction::Signature(id, hash, sig) => TssAction::Signature(id, hash, sig),
		})
	}
}
