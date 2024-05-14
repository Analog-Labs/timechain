use crate::network::PeerId;
use anyhow::Result;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
pub use time_primitives::TssId;
pub use tss::{
	ProofOfKnowledge, Signature, SigningKey, VerifiableSecretSharingCommitment, VerifyingKey,
};

pub type TssMessage = tss::TssMessage<TssId>;

#[derive(Clone)]
pub enum TssAction {
	Send(Vec<(PeerId, TssMessage)>),
	Commit(VerifiableSecretSharingCommitment, ProofOfKnowledge),
	PublicKey(VerifyingKey),
	Signature(TssId, [u8; 32], Signature),
}

pub enum Tss {
	Enabled(tss::Tss<TssId, TssPeerId>),
	Disabled(SigningKey, Option<TssAction>, bool),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TssPeerId(PeerId);

impl TssPeerId {
	pub fn new(peer_id: PeerId) -> Result<Self> {
		peernet::PeerId::from_bytes(&peer_id)?;
		Ok(Self(peer_id))
	}
}

impl From<TssPeerId> for PeerId {
	fn from(p: TssPeerId) -> Self {
		p.0
	}
}

impl tss::ToFrostIdentifier for TssPeerId {
	fn to_frost(&self) -> tss::Identifier {
		tss::Identifier::derive(&self.0).expect("not null")
	}
}

impl std::fmt::Display for TssPeerId {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		peernet::PeerId::from_bytes(&self.0).unwrap().fmt(f)
	}
}

fn signing_share_path(
	keyshare_cache: &Path,
	commitment: &VerifiableSecretSharingCommitment,
) -> PathBuf {
	let file_name = hex::encode(bincode::serialize(commitment).expect("is serializable"));
	keyshare_cache.join(file_name)
}

impl Tss {
	pub fn new(
		peer_id: PeerId,
		members: BTreeSet<PeerId>,
		threshold: u16,
		commitment: Option<VerifiableSecretSharingCommitment>,
		tss_keyshare_cache: &Path,
	) -> Result<Self> {
		let peer_id = TssPeerId::new(peer_id)?;
		let members = members.into_iter().map(TssPeerId::new).collect::<Result<BTreeSet<_>>>()?;
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
			Ok(Tss::Disabled(key, Some(TssAction::Commit(commitment, proof_of_knowledge)), false))
		} else {
			let recover = if let Some(commitment) = commitment {
				let file_name = signing_share_path(tss_keyshare_cache, &commitment);
				let bytes = std::fs::read(file_name)?;
				let signing_share = bincode::deserialize(&bytes)?;
				Some((signing_share, commitment))
			} else {
				None
			};
			Ok(Tss::Enabled(tss::Tss::new(peer_id, members, threshold, recover)))
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
				*actions = Some(TssAction::PublicKey(key.public()));
				*committed = true;
			},
		}
	}

	pub fn on_sign(&mut self, request_id: TssId, data: Vec<u8>) {
		match self {
			Self::Enabled(tss) => tss.on_sign(request_id, data),
			Self::Disabled(key, actions, _) => {
				let hash = VerifyingKey::message_hash(&data);
				*actions = Some(TssAction::Signature(request_id, hash, key.sign_prehashed(hash)));
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
		let peer_id = TssPeerId::new(peer_id)?;
		Ok(match self {
			Self::Enabled(tss) => tss.on_message(peer_id, msg),
			Self::Disabled(_, _, _) => None,
		})
	}

	pub fn next_action(&mut self, tss_keyshare_path: &Path) -> Option<TssAction> {
		let action = match self {
			Self::Enabled(tss) => tss.next_action(),
			Self::Disabled(_, action, _) => return action.take(),
		}?;
		Some(match action {
			tss::TssAction::Send(msgs) => {
				TssAction::Send(msgs.into_iter().map(|(peer, msg)| (peer.into(), msg)).collect())
			},
			tss::TssAction::Commit(commitment, proof_of_knowledge) => {
				TssAction::Commit(commitment, proof_of_knowledge)
			},
			tss::TssAction::Ready(signing_share, commitment, public_key) => {
				let file_name = signing_share_path(tss_keyshare_path, &commitment);
				let bytes =
					bincode::serialize(&signing_share).expect("can serialize signing share");
				#[cfg(unix)]
				{
					use std::{fs::Permissions, os::unix::fs::PermissionsExt};
					std::fs::set_permissions(&file_name, Permissions::from_mode(0o600)).ok();
				}
				std::fs::write(file_name, bytes).ok();
				TssAction::PublicKey(public_key)
			},
			tss::TssAction::Signature(id, hash, sig) => TssAction::Signature(id, hash, sig),
		})
	}
}
