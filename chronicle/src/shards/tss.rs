use crate::network::PeerId;
use std::collections::BTreeSet;
use time_primitives::{AccountId, ShardId};
pub use time_primitives::{TssId, TSS_KEY_PATH};
pub use tss::{
	read_key_from_file, write_key_to_file, SigningKey, VerifiableSecretSharingCommitment,
	VerifyingKey,
};

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
		let (key, committed, is_new) = if commitment.is_some() {
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
			if let Err(e) = write_key_to_file(key.to_bytes().into(), account_id, shard_id) {
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
		Tss::Enabled(tss::Tss::new(peer_id, members, threshold, commitment, account_id, shard_id))
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

	pub fn on_sign(&mut self, request_id: TssId, data: [u8; 32]) {
		match self {
			Self::Enabled(tss) => tss.on_sign(request_id, data.into()),
			Self::Disabled(key, actions, _) => {
				*actions =
					Some(tss::TssAction::Signature(request_id, data, key.sign_prehashed(data)));
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
