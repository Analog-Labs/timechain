use crate::network::PeerId;
use anyhow::Result;
use sp_core::blake2_128;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use time_primitives::{AccountId, ShardId};
pub use time_primitives::{TssId, TSS_KEY_PATH};
pub use tss::{SigningKey, VerifiableSecretSharingCommitment, VerifyingKey};

pub type TssMessage = tss::TssMessage<TssId>;
pub type TssAction = tss::TssAction<TssId, PeerId>;

// read the secret from file stored in local
// name fetched from peer_id and commitment combined.
// Data returned could be SigningKey and SecretShare
fn read_key_from_file(account_id: AccountId, shard_id: ShardId) -> Result<Vec<u8>> {
	let file_path = file_path_from_commitment(account_id, shard_id)?;
	Ok(fs::read(file_path)?)
}

// writes SignignKey to a local file
fn write_key_to_file(key: SigningKey, account_id: AccountId, shard_id: ShardId) -> Result<()> {
	let data = key.to_bytes();
	let file_path = file_path_from_commitment(account_id, shard_id)?;
	fs::write(file_path, data)?;
	Ok(())
}

// Take peer_id and shard commitment
// makes a unique filename using params
fn file_path_from_commitment(account_id: AccountId, shard_id: ShardId) -> Result<PathBuf> {
	let mut combined_data = Vec::new();
	combined_data.extend_from_slice(account_id.as_ref());
	combined_data.extend_from_slice(&shard_id.to_le_bytes());
	let file_name = hex::encode(blake2_128(&combined_data));
	let home_dir = dirs::home_dir().ok_or(anyhow::anyhow!("Home directory not found"))?;
	let analog_dir = home_dir.join(TSS_KEY_PATH);
	fs::create_dir_all(analog_dir.clone())?;
	Ok(analog_dir.join(file_name))
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
		public_key: AccountId,
		shard_id: ShardId,
	) -> Self {
		let peer_id = p2p::PeerId::from_bytes(&peer_id).unwrap().to_string();
		let members: BTreeSet<_> = members
			.into_iter()
			.map(|peer| p2p::PeerId::from_bytes(&peer).unwrap().to_string())
			.collect();
		if members.len() == 1 {
			Self::process_single_member_case(peer_id, commitment, public_key, shard_id)
		} else {
			Self::process_multiple_member_case(
				peer_id, members, threshold, commitment, public_key, shard_id,
			)
		}
	}

	fn process_single_member_case(
		peer_id: String,
		commitment: Option<VerifiableSecretSharingCommitment>,
		account_id: AccountId,
		shard_id: ShardId,
	) -> Self {
		let (key, committed, is_new) = if let Some(_) = commitment {
			match read_key_from_file(account_id.clone(), shard_id) {
				Ok(bytes) if bytes.len() == 32 => {
					let array: [u8; 32] = bytes.try_into().expect("Invalid length");
					match SigningKey::from_bytes(array) {
						Ok(key) => (key, true, false),
						Err(e) => {
							tracing::error!("Failed to create SigningKey from bytes: {}", e);
							(SigningKey::random(), false, true)
						},
					}
				},
				Err(e) => {
					tracing::warn!(
						"Failed to read key from file or invalid key length; using random key {:?}",
						e
					);
					(SigningKey::random(), false, true)
				},
				Ok(_) => (SigningKey::random(), false, false),
			}
		} else {
			(SigningKey::random(), false, true)
		};
		let public = key.public().to_bytes().unwrap();
		let commitment = VerifiableSecretSharingCommitment::deserialize(vec![public]).unwrap();
		let proof_of_knowledge = tss::construct_proof_of_knowledge(
			peer_id.clone(),
			&[*key.to_scalar().as_ref()],
			&commitment,
		)
		.unwrap();
		if is_new {
			if let Err(e) = write_key_to_file(key, account_id, shard_id) {
				tracing::error!("Error writing TSS key to file {}", e);
			};
		}
		Tss::Disabled(key, Some(tss::TssAction::Commit(commitment, proof_of_knowledge)), committed)
	}

	fn process_multiple_member_case(
		peer_id: String,
		members: BTreeSet<String>,
		threshold: u16,
		commitment: Option<VerifiableSecretSharingCommitment>,
		account_id: AccountId,
		shard_id: ShardId,
	) -> Self {
		let commitment = commitment.and_then(|old_commitment| {
			read_key_from_file(account_id, shard_id)
				.ok()
				.map(|secret_share_bytes| (old_commitment, secret_share_bytes))
		});

		Tss::Enabled(tss::Tss::new(peer_id, members, threshold, commitment))
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
