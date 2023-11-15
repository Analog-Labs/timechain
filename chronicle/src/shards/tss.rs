use crate::network::PeerId;
use anyhow::Result;
use std::collections::BTreeSet;
use std::fs;

pub use time_primitives::{TssId, TSS_KEY_PATH};
pub use tss::{SigningKey, VerifiableSecretSharingCommitment, VerifyingKey};

pub type TssMessage = tss::TssMessage<TssId>;
pub type TssAction = tss::TssAction<TssId, PeerId>;

fn read_key_from_file(commitment: VerifiableSecretSharingCommitment) -> Result<Vec<u8>> {
	let serialized_commitment = commitment.serialize();
	if serialized_commitment.len() < 1 {
		return Err(anyhow::anyhow!("Received commitment is empty while recovery TSS state"));
	}
	let file_name = hex::encode(serialized_commitment[0]);
	let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Root directory not found"))?;
	let analog_dir = home_dir.join(TSS_KEY_PATH);
	fs::create_dir_all(analog_dir.clone())?;
	let file_path = analog_dir.join(file_name);
	Ok(fs::read(file_path)?)
}

fn write_key_to_file(key: SigningKey, commitment: VerifiableSecretSharingCommitment) -> Result<()> {
	let serialized_commitment = commitment.serialize();
	if serialized_commitment.len() < 1 {
		return Err(anyhow::anyhow!("Received commitment is empty while recovery TSS state"));
	}
	let data = key.to_bytes();
	let file_name = hex::encode(serialized_commitment[0]);
	let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Root directory not found"))?;
	let analog_dir = home_dir.join(TSS_KEY_PATH);
	fs::create_dir_all(analog_dir.clone())?;
	let file_path = analog_dir.join(file_name);
	fs::write(file_path, data)?;
	Ok(())
}

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
	) -> Self {
		let peer_id = p2p::PeerId::from_bytes(&peer_id).unwrap().to_string();
		let members: BTreeSet<_> = members
			.into_iter()
			.map(|peer| p2p::PeerId::from_bytes(&peer).unwrap().to_string())
			.collect();
		if members.len() == 1 {
			let (key, committed) = if let Some(old_commitment) = commitment {
				let bytes = read_key_from_file(old_commitment).unwrap();
				(SigningKey::from_bytes(bytes.try_into().unwrap()).unwrap(), true)
			} else {
				(SigningKey::random(), false)
			};
			let public = key.public().to_bytes().unwrap();
			let commitment = VerifiableSecretSharingCommitment::deserialize(vec![public]).unwrap();
			let proof_of_knowledge = tss::construct_proof_of_knowledge(
				peer_id,
				&[*key.to_scalar().as_ref()],
				&commitment,
			)
			.unwrap();
			if let Err(e) = write_key_to_file(key, commitment.clone()) {
				tracing::error!("Error writing TSS key to file {}", e);
			};
			Tss::Disabled(
				key,
				Some(tss::TssAction::Commit(commitment, proof_of_knowledge)),
				committed,
			)
		} else if let Some(old_commitment) = commitment {
			let secret_share_bytes = read_key_from_file(old_commitment.clone()).unwrap();
			Tss::Enabled(tss::Tss::new(
				peer_id,
				members,
				threshold,
				Some((old_commitment, secret_share_bytes)),
			))
		} else {
			Tss::Enabled(tss::Tss::new(peer_id, members, threshold, None))
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

	pub fn on_message(&mut self, peer_id: PeerId, msg: TssMessage) -> Option<TssMessage> {
		let peer_id = p2p::PeerId::from_bytes(&peer_id).unwrap().to_string();
		match self {
			Self::Enabled(tss) => tss.on_message(peer_id, msg),
			Self::Disabled(_, _, _) => None,
		}
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
						let peer: p2p::PeerId = peer.parse().unwrap();
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
