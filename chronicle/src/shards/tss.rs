use crate::network::PeerId;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha3::{Digest, Sha3_256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
pub use time_primitives::TaskId;
pub use tss::{
	ProofOfKnowledge, Signature, SigningKey, VerifiableSecretSharingCommitment, VerifyingKey,
};

pub type TssMessage = tss::TssMessage<TaskId>;

#[derive(Clone)]
pub enum TssAction {
	Send(Vec<(PeerId, TssMessage)>),
	Commit(VerifiableSecretSharingCommitment, ProofOfKnowledge),
	PublicKey(VerifyingKey),
	Signature(TaskId, [u8; 32], Signature),
}

pub enum Tss {
	Enabled(tss::Tss<TaskId, TssPeerId>),
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
	let bytes = bincode::serialize(commitment).expect("is serializable");
	let mut hasher = Sha3_256::new();
	hasher.update(&bytes);
	let hash = hasher.finalize();
	let file_name = hex::encode(hash);
	keyshare_cache.join(file_name)
}

fn read_signing_share<T: DeserializeOwned>(
	keyshare_cache: &Path,
	commitment: &VerifiableSecretSharingCommitment,
) -> Result<T> {
	let file_name = signing_share_path(keyshare_cache, commitment);
	let bytes = std::fs::read(file_name)?;
	Ok(bincode::deserialize(&bytes)?)
}

fn write_signing_share<T: Serialize>(
	keyshare_cache: &Path,
	commitment: &VerifiableSecretSharingCommitment,
	signing_share: &T,
) {
	let file_name = signing_share_path(keyshare_cache, commitment);
	let bytes = bincode::serialize(signing_share).expect("can serialize signing share");
	#[cfg(unix)]
	{
		use std::{fs::Permissions, os::unix::fs::PermissionsExt};
		std::fs::set_permissions(&file_name, Permissions::from_mode(0o600)).ok();
	}
	if let Err(err) = std::fs::write(file_name, bytes) {
		tracing::error!("failed to write to tss cache directory {:#?}", err);
	}
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
			if let Some(commitment) = commitment {
				let key = read_signing_share(tss_keyshare_cache, &commitment)?;
				let key = SigningKey::from_bytes(key)?;
				Ok(Tss::Disabled(key, None, true))
			} else {
				let key = SigningKey::random();
				let public = key.public().to_bytes().unwrap();
				let commitment =
					VerifiableSecretSharingCommitment::deserialize(vec![public]).unwrap();
				let proof_of_knowledge = tss::construct_proof_of_knowledge(
					peer_id,
					&[*key.to_scalar().as_ref()],
					&commitment,
				)
				.unwrap();
				write_signing_share(tss_keyshare_cache, &commitment, &key.to_bytes());
				Ok(Tss::Disabled(
					key,
					Some(TssAction::Commit(commitment, proof_of_knowledge)),
					false,
				))
			}
		} else {
			let recover = if let Some(commitment) = commitment {
				Some(read_signing_share(tss_keyshare_cache, &commitment)?)
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

	pub fn on_start(&mut self, request_id: TaskId) {
		match self {
			Self::Enabled(tss) => tss.on_start(request_id),
			Self::Disabled(_, _, _) => {},
		}
	}

	pub fn on_sign(&mut self, request_id: TaskId, data: Vec<u8>) {
		match self {
			Self::Enabled(tss) => tss.on_sign(request_id, data),
			Self::Disabled(key, actions, _) => {
				let hash = VerifyingKey::message_hash(&data);
				*actions = Some(TssAction::Signature(request_id, hash, key.sign_prehashed(hash)));
			},
		}
	}

	pub fn on_complete(&mut self, request_id: TaskId) {
		match self {
			Self::Enabled(tss) => tss.on_complete(request_id),
			Self::Disabled(_, _, _) => {},
		}
	}

	pub fn on_message(&mut self, peer_id: PeerId, msg: TssMessage) -> Result<()> {
		let peer_id = TssPeerId::new(peer_id)?;
		match self {
			Self::Enabled(tss) => tss.on_message(peer_id, msg)?,
			Self::Disabled(_, _, _) => {},
		};
		Ok(())
	}

	pub fn next_action(&mut self, tss_keyshare_cache: &Path) -> Option<TssAction> {
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
				write_signing_share(tss_keyshare_cache, &commitment, &signing_share);
				TssAction::PublicKey(public_key)
			},
			tss::TssAction::Signature(id, hash, sig) => TssAction::Signature(id, hash, sig),
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn test_tss_recovery(n: usize) {
		let dir = tempfile::tempdir().unwrap();
		let mut members = BTreeSet::new();
		for i in 1..(n + 1) {
			let secret = [i as u8; 32];
			let peerid = ed25519_dalek::SigningKey::from_bytes(&secret).verifying_key().to_bytes();
			members.insert(peerid);
		}
		let peerid = *members.iter().next().unwrap();
		let mut tss = Tss::new(peerid, members.clone(), n as _, None, dir.path()).unwrap();
		let TssAction::Commit(commitment, _) = tss.next_action(dir.path()).unwrap() else {
			panic!();
		};
		Tss::new(peerid, members, n as _, Some(commitment), dir.path()).unwrap();
	}

	#[test]
	fn test_tss_recovery_1() {
		test_tss_recovery(1);
	}
}
