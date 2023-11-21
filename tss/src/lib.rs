#![allow(clippy::large_enum_variant)]
use crate::dkg::{Dkg, DkgAction, DkgMessage};
use crate::roast::{Roast, RoastAction, RoastRequest, RoastSignerResponse};
use crate::rts::{Rts, RtsAction, RtsHelper, RtsRequest, RtsResponse};
use anyhow::Result;
use frost_evm::keys::{KeyPackage, PublicKeyPackage, SecretShare};
use frost_evm::{Identifier, Scalar};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sp_core::blake2_128;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use time_primitives::{AccountId, ShardId, TSS_KEY_PATH};

pub use frost_evm::frost_core::frost::keys::sum_commitments;
pub use frost_evm::frost_secp256k1::Signature as ProofOfKnowledge;
pub use frost_evm::keys::VerifiableSecretSharingCommitment;
pub use frost_evm::schnorr::SigningKey;
pub use frost_evm::{Signature, VerifyingKey};

mod dkg;
mod roast;
mod rts;
#[cfg(test)]
mod tests;

enum TssState<I> {
	Dkg(Dkg),
	_Rts(Rts),
	Roast {
		rts: RtsHelper,
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		signing_sessions: BTreeMap<I, Roast>,
	},
	Failed,
}

#[derive(Clone)]
pub enum TssAction<I, P> {
	Send(Vec<(P, TssMessage<I>)>),
	Commit(VerifiableSecretSharingCommitment, ProofOfKnowledge),
	PublicKey(VerifyingKey),
	Signature(I, [u8; 32], Signature),
}

#[derive(Clone, Deserialize, Serialize)]
pub enum TssMessage<I> {
	Request(TssRequest<I>),
	Response(TssResponse<I>),
}

impl<I: std::fmt::Display> std::fmt::Display for TssMessage<I> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Request(msg) => write!(f, "req {}", msg),
			Self::Response(msg) => write!(f, "rsp {}", msg),
		}
	}
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub enum TssRequest<I> {
	Dkg { msg: DkgMessage },
	Rts { msg: RtsRequest },
	Roast { id: I, msg: RoastRequest },
}

impl<I: std::fmt::Display> std::fmt::Display for TssRequest<I> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Dkg { msg } => write!(f, "dkg {}", msg),
			Self::Rts { msg } => write!(f, "rts {}", msg),
			Self::Roast { id, msg } => write!(f, "roast {} {}", id, msg),
		}
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub enum TssResponse<I> {
	Rts { msg: RtsResponse },
	Roast { id: I, msg: RoastSignerResponse },
}

impl<I: std::fmt::Display> std::fmt::Display for TssResponse<I> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Rts { msg } => write!(f, "rts {}", msg),
			Self::Roast { id, .. } => write!(f, "roast {}", id),
		}
	}
}

fn peer_to_frost(peer: impl std::fmt::Display) -> Identifier {
	Identifier::derive(peer.to_string().as_bytes()).expect("non zero")
}

pub fn construct_proof_of_knowledge(
	peer: impl std::fmt::Display,
	coefficients: &[Scalar],
	commitment: &VerifiableSecretSharingCommitment,
) -> Result<ProofOfKnowledge> {
	let identifier = peer_to_frost(peer);
	Ok(frost_evm::frost_core::frost::keys::dkg::compute_proof_of_knowledge(
		identifier,
		coefficients,
		commitment,
		OsRng,
	)?)
}

pub fn verify_proof_of_knowledge(
	peer: impl std::fmt::Display,
	commitment: &VerifiableSecretSharingCommitment,
	proof_of_knowledge: ProofOfKnowledge,
) -> Result<()> {
	let identifier = peer_to_frost(peer);
	Ok(frost_evm::frost_core::frost::keys::dkg::verify_proof_of_knowledge(
		identifier,
		commitment,
		proof_of_knowledge,
	)?)
}

// read the secret from file stored in local
// name fetched from peer_id and commitment combined.
// Data returned could be SigningKey and SecretShare
pub fn read_key_from_file(account_id: AccountId, shard_id: ShardId) -> Result<Vec<u8>> {
	let file_path = file_path_from_commitment(account_id, shard_id).unwrap();
	let file = File::open(file_path)?;
	let reader = BufReader::new(file);
	let data: Vec<u8> = serde_json::from_reader(reader)?;
	Ok(data)
}

// writes SignignKey to a local file
pub fn write_key_to_file(key: Vec<u8>, account_id: AccountId, shard_id: ShardId) -> Result<()> {
	let file_path = file_path_from_commitment(account_id, shard_id).unwrap();
	let mut file = File::create(file_path).unwrap();

	#[cfg(target_family = "unix")]
	{
		use std::os::unix::fs::PermissionsExt;
		file.set_permissions(fs::Permissions::from_mode(0o600))?;
	}

	serde_json::to_writer(&file, &key).unwrap();
	file.flush().unwrap();
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

/// Tss state machine.
pub struct Tss<I, P> {
	peer_id: P,
	frost_id: Identifier,
	frost_to_peer: BTreeMap<Identifier, P>,
	threshold: u16,
	coordinators: BTreeSet<Identifier>,
	state: TssState<I>,
	committed: bool,
	account_id: AccountId,
	shard_id: ShardId,
}

impl<I, P> Tss<I, P>
where
	I: Clone + Ord + std::fmt::Display,
	P: Clone + Ord + std::fmt::Display,
{
	pub fn new(
		peer_id: P,
		members: BTreeSet<P>,
		threshold: u16,
		commitment: Option<VerifiableSecretSharingCommitment>,
		account_id: AccountId,
		shard_id: ShardId,
	) -> Self {
		debug_assert!(members.contains(&peer_id));
		let frost_id = peer_to_frost(&peer_id);
		let frost_to_peer: BTreeMap<_, _> =
			members.into_iter().map(|peer| (peer_to_frost(&peer), peer)).collect();
		let members: BTreeSet<_> = frost_to_peer.keys().copied().collect();
		let coordinators: BTreeSet<_> =
			members.iter().copied().take(members.len() - threshold as usize + 1).collect();
		let is_coordinator = coordinators.contains(&frost_id);
		tracing::debug!(
			"{} initialize {}/{} coordinator = {}",
			peer_id,
			threshold,
			members.len(),
			is_coordinator
		);
		let (state, committed) =
			Self::get_state(frost_id, members, threshold, commitment, account_id.clone(), shard_id);
		Self {
			peer_id,
			frost_id,
			frost_to_peer,
			threshold,
			coordinators,
			state,
			committed,
			account_id,
			shard_id,
		}
	}

	fn get_state(
		frost_id: Identifier,
		members: BTreeSet<Identifier>,
		threshold: u16,
		commitment: Option<VerifiableSecretSharingCommitment>,
		account_id: AccountId,
		shard_id: ShardId,
	) -> (TssState<I>, bool) {
		if let Some(commitment) = commitment {
			let Ok(secret_bytes) = read_key_from_file(account_id, shard_id) else {
				tracing::error!("Could not fetch secret key from local");
				return (TssState::Dkg(Dkg::new(frost_id, members, threshold)), false);
			};
			let Ok(secret_share) = SecretShare::deserialize(&secret_bytes) else {
				tracing::error!("Could not parse secret_share");
				return (TssState::Dkg(Dkg::new(frost_id, members, threshold)), false);
			};
			let rts = RtsHelper::new(frost_id, members.clone(), threshold, secret_share.clone());
			let Ok(key_package) = KeyPackage::try_from(secret_share) else {
				tracing::info!("Could not make keypackage from secret_share");
				return (TssState::Dkg(Dkg::new(frost_id, members, threshold)), false);
			};
			let Ok(public_key_package) = PublicKeyPackage::from_commitment(&members, &commitment)
			else {
				tracing::info!("Could not recover public key from commitment");
				return (TssState::Dkg(Dkg::new(frost_id, members, threshold)), false);
			};
			let state = TssState::<I>::Roast {
				rts,
				key_package,
				public_key_package,
				signing_sessions: Default::default(),
			};
			(state, true)
		} else {
			tracing::debug!("commitment not received for recovery");
			(TssState::Dkg(Dkg::new(frost_id, members, threshold)), false)
		}
	}

	pub fn peer_id(&self) -> &P {
		&self.peer_id
	}

	fn frost_to_peer(&self, frost: &Identifier) -> P {
		self.frost_to_peer.get(frost).unwrap().clone()
	}

	pub fn total_nodes(&self) -> usize {
		self.frost_to_peer.len()
	}

	pub fn threshold(&self) -> usize {
		self.threshold as _
	}

	pub fn committed(&self) -> bool {
		self.committed
	}

	pub fn on_message(&mut self, peer_id: P, msg: TssMessage<I>) -> Option<TssMessage<I>> {
		match msg {
			TssMessage::Request(request) => match self.on_request(peer_id, request) {
				Ok(Some(response)) => Some(TssMessage::Response(response)),
				Ok(None) => None,
				Err(error) => {
					tracing::info!("received invalid request: {:?}", error);
					None
				},
			},
			TssMessage::Response(response) => {
				self.on_response(peer_id, response);
				None
			},
		}
	}

	fn on_request(&mut self, peer_id: P, request: TssRequest<I>) -> Result<Option<TssResponse<I>>> {
		tracing::debug!("{} on_request {} {}", self.peer_id, peer_id, request);
		if self.peer_id == peer_id {
			anyhow::bail!("{} received message from self", self.peer_id);
		}
		let frost_id = peer_to_frost(&peer_id);
		if !self.frost_to_peer.contains_key(&frost_id) {
			anyhow::bail!("{} received message unknown peer {}", self.peer_id, peer_id);
		}
		match (&mut self.state, request) {
			(TssState::Dkg(dkg), TssRequest::Dkg { msg }) => {
				dkg.on_message(frost_id, msg);
				Ok(None)
			},
			(TssState::Roast { rts, .. }, TssRequest::Rts { msg }) => {
				let msg = rts.on_request(frost_id, msg)?;
				Ok(Some(TssResponse::Rts { msg }))
			},
			(TssState::Roast { signing_sessions, .. }, TssRequest::Roast { id, msg }) => {
				if let Some(session) = signing_sessions.get_mut(&id) {
					if let Some(msg) = session.on_request(frost_id, msg)? {
						Ok(Some(TssResponse::Roast { id, msg }))
					} else {
						Ok(None)
					}
				} else {
					anyhow::bail!("invalid signing session");
				}
			},
			(_, msg) => {
				anyhow::bail!("unexpected request {}", msg);
			},
		}
	}

	fn on_response(&mut self, peer_id: P, response: TssResponse<I>) {
		let frost_id = peer_to_frost(&peer_id);
		match (&mut self.state, response) {
			(TssState::Dkg(_), _) => {},
			(TssState::_Rts(rts), TssResponse::Rts { msg }) => {
				rts.on_response(frost_id, Some(msg));
				// TODO: make rts asynchronous
				// rts.on_response(frost_id, None);
			},
			(TssState::Roast { signing_sessions, .. }, TssResponse::Roast { id, msg }) => {
				if let Some(session) = signing_sessions.get_mut(&id) {
					session.on_response(frost_id, msg);
				} else {
					tracing::error!("invalid signing session");
				}
			},
			(_, msg) => {
				tracing::error!("invalid state ({}, {}, {})", self.peer_id, peer_id, msg);
			},
		}
	}

	pub fn on_commit(&mut self, commitment: VerifiableSecretSharingCommitment) {
		tracing::debug!("{} commit", self.peer_id);
		match &mut self.state {
			TssState::Dkg(dkg) => {
				dkg.on_commit(commitment);
				self.committed = true;
			},
			_ => tracing::error!("unexpected commit"),
		}
	}

	pub fn on_sign(&mut self, id: I, data: Vec<u8>) {
		tracing::debug!("{} sign {}", self.peer_id, id);
		match &mut self.state {
			TssState::Roast {
				key_package,
				public_key_package,
				signing_sessions,
				..
			} => {
				let roast = Roast::new(
					self.frost_id,
					self.threshold,
					key_package.clone(),
					public_key_package.clone(),
					data,
					self.coordinators.clone(),
				);
				signing_sessions.insert(id, roast);
			},
			_ => {
				tracing::error!("not ready to sign");
			},
		}
	}

	pub fn on_complete(&mut self, id: I) {
		tracing::debug!("{} complete {}", self.peer_id, id);
		match &mut self.state {
			TssState::Roast { signing_sessions, .. } => {
				signing_sessions.remove(&id);
			},
			_ => {
				tracing::error!("not ready to complete");
			},
		}
	}

	pub fn next_action(&mut self) -> Option<TssAction<I, P>> {
		match &mut self.state {
			TssState::Dkg(dkg) => {
				match dkg.next_action()? {
					DkgAction::Send(msgs) => {
						return Some(TssAction::Send(
							msgs.into_iter()
								.map(|(peer, msg)| {
									(
										self.frost_to_peer(&peer),
										TssMessage::Request(TssRequest::Dkg { msg }),
									)
								})
								.collect(),
						));
					},
					DkgAction::Commit(commitment, proof_of_knowledge) => {
						return Some(TssAction::Commit(commitment, proof_of_knowledge));
					},
					DkgAction::Complete(key_package, public_key_package, commitment) => {
						let secret_share = SecretShare::new(
							self.frost_id,
							*key_package.signing_share(),
							commitment.clone(),
						);
						if let Err(e) = write_key_to_file(
							secret_share.clone().serialize().unwrap(),
							self.account_id.clone(),
							self.shard_id,
						) {
							tracing::error!("error saving secret {:?}", e);
						};
						let public_key =
							VerifyingKey::new(public_key_package.verifying_key().to_element());
						let members = self.frost_to_peer.keys().copied().collect();
						let rts =
							RtsHelper::new(self.frost_id, members, self.threshold, secret_share);
						self.state = TssState::Roast {
							rts,
							key_package,
							public_key_package,
							signing_sessions: Default::default(),
						};
						return Some(TssAction::PublicKey(public_key));
					},
					DkgAction::Failure(error) => {
						tracing::error!("dkg failed with {:?}", error);
						self.state = TssState::Failed;
						return None;
					},
				};
			},
			TssState::_Rts(rts) => match rts.next_action()? {
				RtsAction::Send(msgs) => {
					return Some(TssAction::Send(
						msgs.into_iter()
							.map(|(peer, msg)| {
								(
									self.frost_to_peer(&peer),
									TssMessage::Request(TssRequest::Rts { msg }),
								)
							})
							.collect(),
					));
				},
				RtsAction::Complete(key_package, public_key_package, commitment) => {
					let secret_share =
						SecretShare::new(self.frost_id, *key_package.signing_share(), commitment);
					let public_key =
						VerifyingKey::new(public_key_package.verifying_key().to_element());
					let members = self.frost_to_peer.keys().copied().collect();
					let rts = RtsHelper::new(self.frost_id, members, self.threshold, secret_share);
					self.state = TssState::Roast {
						rts,
						key_package,
						public_key_package,
						signing_sessions: Default::default(),
					};
					return Some(TssAction::PublicKey(public_key));
				},
				RtsAction::Failure => {
					self.state = TssState::Failed;
					return None;
				},
			},
			TssState::Roast { signing_sessions, .. } => {
				let session_ids: Vec<_> = signing_sessions.keys().cloned().collect();
				for id in session_ids {
					let session = signing_sessions.get_mut(&id).unwrap();
					while let Some(action) = session.next_action() {
						let (peers, send_to_self, msg) = match action {
							RoastAction::Send(peer, msg) => {
								if peer == self.frost_id {
									(vec![], true, msg)
								} else {
									(vec![peer], false, msg)
								}
							},
							RoastAction::SendMany(all_peers, msg) => {
								let peers: Vec<_> = all_peers
									.iter()
									.filter(|peer| **peer != self.frost_id)
									.copied()
									.collect();
								let send_to_self = peers.len() != all_peers.len();
								(peers, send_to_self, msg)
							},
							RoastAction::Complete(hash, signature) => {
								return Some(TssAction::Signature(id, hash, signature));
							},
						};
						if send_to_self {
							if let Some(response) = session
								.on_request(self.frost_id, msg.clone())
								.expect("something wrong")
							{
								session.on_response(self.frost_id, response);
							}
						}
						if !peers.is_empty() {
							return Some(TssAction::Send(
								peers
									.into_iter()
									.map(|peer| {
										(
											self.frost_to_peer(&peer),
											TssMessage::Request(TssRequest::Roast {
												id: id.clone(),
												msg: msg.clone(),
											}),
										)
									})
									.collect(),
							));
						}
					}
				}
			},
			TssState::Failed => {},
		}
		None
	}
}
