#![allow(clippy::large_enum_variant)]
use frost_evm::{
	keys::{dkg, KeyPackage, PublicKeyPackage},
	round1::{self, SigningCommitments, SigningNonces},
	round2::{self, SignatureShare},
	Error, Identifier, Signature, SigningPackage, VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Tss state.
enum TssState<P> {
	Uninitialized {
		round1_packages: BTreeMap<P, dkg::round1::Package>,
	},
	DkgR1 {
		secret_package: Option<dkg::round1::SecretPackage>,
		round1_packages: BTreeMap<P, dkg::round1::Package>,
		round2_packages: BTreeMap<P, dkg::round2::Package>,
	},
	DkgR2 {
		secret_package: dkg::round2::SecretPackage,
		round1_packages: Vec<dkg::round1::Package>,
		round2_packages: BTreeMap<P, dkg::round2::Package>,
	},
	Initialized {
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		signing_state: BTreeMap<[u8; 32], SigningState<P>>,
	},
}

impl<P> Default for TssState<P> {
	fn default() -> Self {
		Self::Uninitialized {
			round1_packages: Default::default(),
		}
	}
}

impl<P> std::fmt::Display for TssState<P> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Uninitialized { round1_packages } => {
				write!(f, "uninitialized {}", round1_packages.len())
			},
			Self::DkgR1 { round1_packages, .. } => write!(f, "dkgr1 {}", round1_packages.len()),
			Self::DkgR2 { round2_packages, .. } => write!(f, "dkgr2 {}", round2_packages.len()),
			Self::Initialized { .. } => write!(f, "initialized"),
		}
	}
}

enum SigningState<P> {
	PreCommit {
		commitments: BTreeMap<P, SigningCommitments>,
	},
	Commit {
		data: Vec<u8>,
		nonces: SigningNonces,
		commitments: BTreeMap<P, SigningCommitments>,
		signature_shares: BTreeMap<P, SignatureShare>,
	},
	Sign {
		signing_package: SigningPackage,
		signature_shares: BTreeMap<P, SignatureShare>,
	},
}

impl<P> Default for SigningState<P> {
	fn default() -> Self {
		Self::PreCommit {
			commitments: Default::default(),
		}
	}
}

impl<P> std::fmt::Display for SigningState<P> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::PreCommit { commitments } => write!(f, "precommit {}", commitments.len()),
			Self::Commit { commitments, .. } => write!(f, "commit {}", commitments.len()),
			Self::Sign { signature_shares, .. } => write!(f, "sign {}", signature_shares.len()),
		}
	}
}

#[derive(Clone)]
pub enum TssAction<P> {
	Send(TssMessage),
	PublicKey(VerifyingKey),
	Tss(Signature),
	Report(P),
	Timeout(Timeout),
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub enum TssMessage {
	DkgR1 { round1_package: dkg::round1::Package },
	DkgR2 { round2_package: Vec<dkg::round2::Package> },
	Sign { hash: [u8; 32], msg: SigningMessage },
}

impl std::fmt::Display for TssMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::DkgR1 { .. } => write!(f, "dkgr1"),
			Self::DkgR2 { .. } => write!(f, "dkgr2"),
			Self::Sign { .. } => write!(f, "sign"),
		}
	}
}

#[derive(Clone, Deserialize, Serialize)]
pub enum SigningMessage {
	Commit { commitment: SigningCommitments },
	Sign { signature_share: SignatureShare },
}

impl std::fmt::Display for SigningMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Commit { .. } => write!(f, "commit"),
			Self::Sign { .. } => write!(f, "sign"),
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timeout(TimeoutKind);

impl Timeout {
	const UNINITIALIZED: Self = Self(TimeoutKind::Uninitialized);
	const DKGR1: Self = Self(TimeoutKind::DkgR1);
	const DKGR2: Self = Self(TimeoutKind::DkgR2);
	const fn precommit(hash: [u8; 32]) -> Self {
		Self(TimeoutKind::Sign(hash, SignTimeoutKind::PreCommit))
	}
	const fn commit(hash: [u8; 32]) -> Self {
		Self(TimeoutKind::Sign(hash, SignTimeoutKind::Commit))
	}
	const fn sign(hash: [u8; 32]) -> Self {
		Self(TimeoutKind::Sign(hash, SignTimeoutKind::Sign))
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimeoutKind {
	Uninitialized,
	DkgR1,
	DkgR2,
	Sign([u8; 32], SignTimeoutKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignTimeoutKind {
	PreCommit,
	Commit,
	Sign,
}

fn validate_dkg_round2_package<P>(
	sender_identifier: Identifier,
	self_identifier: &Identifier,
	receivers: &BTreeMap<Identifier, P>,
	round2_package: Vec<dkg::round2::Package>,
) -> Result<dkg::round2::Package, ()> {
	let mut packages = BTreeMap::new();
	for round2_package in round2_package {
		if round2_package.sender_identifier != sender_identifier {
			return Err(());
		}
		if round2_package.receiver_identifier == sender_identifier {
			return Err(());
		}
		if !receivers.contains_key(&round2_package.receiver_identifier) {
			return Err(());
		}
		packages.insert(round2_package.receiver_identifier, round2_package);
	}
	if packages.len() != receivers.len() - 1 {
		return Err(());
	}
	Ok(packages.remove(self_identifier).unwrap())
}

struct TssConfig<P> {
	peer_to_frost: BTreeMap<P, Identifier>,
	frost_to_peer: BTreeMap<Identifier, P>,
	threshold: usize,
	total_nodes: usize,
}

impl<P> Default for TssConfig<P> {
	fn default() -> Self {
		Self {
			peer_to_frost: Default::default(),
			frost_to_peer: Default::default(),
			threshold: 0,
			total_nodes: 0,
		}
	}
}

impl<P: Clone + Ord> TssConfig<P> {
	pub fn new(peer_id: &P, members: BTreeSet<P>, threshold: u16) -> Self {
		debug_assert!(members.contains(peer_id));
		let threshold = threshold as _;
		let total_nodes = members.len();
		let peer_to_frost: BTreeMap<_, _> = members
			.into_iter()
			.enumerate()
			.map(|(i, peer_id)| {
				let frost = Identifier::try_from(i as u16 + 1).expect("non zero");
				(peer_id, frost)
			})
			.collect();
		let frost_to_peer =
			peer_to_frost.iter().map(|(peer_id, frost)| (*frost, peer_id.clone())).collect();
		Self {
			peer_to_frost,
			frost_to_peer,
			threshold,
			total_nodes,
		}
	}
}

/// Tss state machine.
pub struct Tss<P> {
	peer_id: P,
	frost_id: Identifier,
	config: TssConfig<P>,
	state: TssState<P>,
	actions: VecDeque<TssAction<P>>,
}

impl<P: Clone + Ord + std::fmt::Display> Tss<P> {
	pub fn new(peer_id: P) -> Self {
		Self {
			peer_id,
			frost_id: Identifier::try_from(1).unwrap(),
			config: Default::default(),
			state: Default::default(),
			actions: Default::default(),
		}
	}

	pub fn peer_id(&self) -> &P {
		&self.peer_id
	}

	fn peer_to_frost(&self, peer: &P) -> Identifier {
		*self.config.peer_to_frost.get(peer).unwrap()
	}

	fn frost_to_peer(&self, frost: &Identifier) -> P {
		self.config.frost_to_peer.get(frost).unwrap().clone()
	}

	pub fn total_nodes(&self) -> usize {
		self.config.total_nodes
	}

	pub fn threshold(&self) -> usize {
		self.config.threshold
	}

	fn report(&mut self, frost: Identifier) {
		self.state = TssState::default();
		let peer = self.frost_to_peer(&frost);
		self.actions.push_back(TssAction::Report(peer));
	}

	pub fn initialize(&mut self, members: BTreeSet<P>, threshold: u16) {
		log::debug!("{} initialize {}/{}", self.peer_id, threshold, members.len());
		let round1_packages_preinit = match &mut self.state {
			TssState::Uninitialized { round1_packages } => std::mem::take(round1_packages),
			state => panic!("invalid state ({})", state),
		};
		self.config = TssConfig::new(&self.peer_id, members, threshold);
		self.frost_id = self.peer_to_frost(&self.peer_id);
		let (secret_package, round1_package) = dkg::part1(
			self.frost_id,
			self.config.total_nodes as _,
			self.config.threshold as _,
			OsRng,
		)
		.expect("valid inputs");
		self.state = TssState::DkgR1 {
			secret_package: Some(secret_package),
			round1_packages: Default::default(),
			round2_packages: Default::default(),
		};
		self.actions.push_back(TssAction::Timeout(Timeout::DKGR1));
		self.actions.push_back(TssAction::Send(TssMessage::DkgR1 { round1_package }));
		for (peer_id, round1_package) in round1_packages_preinit {
			self.on_message(peer_id, TssMessage::DkgR1 { round1_package });
		}
	}

	pub fn on_timeout(&mut self, timeout: Timeout) {
		let mut report = vec![];
		match (&self.state, timeout.0) {
			(TssState::Uninitialized { round1_packages }, TimeoutKind::Uninitialized) => {
				for (peer_id, frost_id) in &self.config.peer_to_frost {
					if round1_packages.contains_key(peer_id) {
						report.push(*frost_id);
					}
				}
			},
			(TssState::DkgR1 { round1_packages, .. }, TimeoutKind::DkgR1) => {
				for (peer_id, frost_id) in &self.config.peer_to_frost {
					if peer_id == &self.peer_id {
						continue;
					}
					if !round1_packages.contains_key(peer_id) {
						report.push(*frost_id);
					}
				}
			},
			(TssState::DkgR2 { round2_packages, .. }, TimeoutKind::DkgR2) => {
				for (peer_id, frost_id) in &self.config.peer_to_frost {
					if peer_id == &self.peer_id {
						continue;
					}
					if !round2_packages.contains_key(peer_id) {
						report.push(*frost_id);
					}
				}
			},
			(TssState::Initialized { signing_state, .. }, TimeoutKind::Sign(hash, timeout)) => {
				match (signing_state.get(&hash), timeout) {
					(Some(SigningState::PreCommit { commitments }), SignTimeoutKind::PreCommit) => {
						for (peer_id, frost_id) in &self.config.peer_to_frost {
							if commitments.contains_key(peer_id) {
								report.push(*frost_id);
							}
						}
					},
					(Some(SigningState::Commit { commitments, .. }), SignTimeoutKind::Commit) => {
						for (peer_id, frost_id) in &self.config.peer_to_frost {
							if !commitments.contains_key(peer_id) {
								report.push(*frost_id);
							}
						}
					},
					(Some(SigningState::Sign { signature_shares, .. }), SignTimeoutKind::Sign) => {
						for (peer_id, frost_id) in &self.config.peer_to_frost {
							if !signature_shares.contains_key(peer_id) {
								report.push(*frost_id);
							}
						}
					},
					_ => {},
				}
			},
			_ => {},
		}
		for report in report {
			self.report(report);
		}
	}

	pub fn on_message(&mut self, peer_id: P, msg: TssMessage) {
		if self.peer_id == peer_id {
			return;
		}
		if self.config.peer_to_frost.is_empty() {
			match (&mut self.state, msg) {
				(
					TssState::Uninitialized { round1_packages },
					TssMessage::DkgR1 { round1_package },
				) => {
					round1_packages.insert(peer_id, round1_package);
					self.actions.push_back(TssAction::Timeout(Timeout::UNINITIALIZED));
				},
				(state, msg) => {
					log::error!(
						"invalid state ({}, {}, {}, {})",
						self.peer_id,
						state,
						peer_id,
						msg
					);
				},
			}
			return;
		}
		if !self.config.peer_to_frost.contains_key(&peer_id) {
			log::info!("{} dropping message from {}", self.peer_id, peer_id);
			return;
		}
		log::debug!("{} on_message {} {}", self.peer_id, peer_id, msg);
		let frost_id = self.peer_to_frost(&peer_id);
		match (&mut self.state, msg) {
			(TssState::DkgR1 { round1_packages, .. }, TssMessage::DkgR1 { round1_package, .. }) => {
				if round1_package.sender_identifier != frost_id {
					self.report(frost_id);
					return;
				}
				round1_packages.insert(peer_id, round1_package);
				self.transition(None);
			},
			(TssState::DkgR1 { round2_packages, .. }, TssMessage::DkgR2 { round2_package, .. }) => {
				match validate_dkg_round2_package(
					frost_id,
					&self.frost_id,
					&self.config.frost_to_peer,
					round2_package,
				) {
					Ok(round2_package) => {
						round2_packages.insert(peer_id, round2_package);
					},
					Err(()) => self.report(frost_id),
				};
			},
			(TssState::DkgR2 { round2_packages, .. }, TssMessage::DkgR2 { round2_package, .. }) => {
				match validate_dkg_round2_package(
					frost_id,
					&self.frost_id,
					&self.config.frost_to_peer,
					round2_package,
				) {
					Ok(round2_package) => {
						round2_packages.insert(peer_id, round2_package);
					},
					Err(()) => self.report(frost_id),
				};
				self.transition(None);
			},
			(TssState::Initialized { signing_state, .. }, TssMessage::Sign { hash, msg }) => {
				if !signing_state.contains_key(&hash) {
					self.actions.push_back(TssAction::Timeout(Timeout::precommit(hash)));
				}
				let state = signing_state.entry(hash).or_default();
				match (state, msg) {
					(
						SigningState::PreCommit { commitments },
						SigningMessage::Commit { commitment },
					) => {
						if commitment.identifier != frost_id {
							self.report(frost_id);
							return;
						}
						commitments.insert(peer_id, commitment);
					},
					(
						SigningState::Commit { commitments, .. },
						SigningMessage::Commit { commitment },
					) => {
						if commitment.identifier != frost_id {
							self.report(frost_id);
							return;
						}
						commitments.insert(peer_id, commitment);
						self.transition(Some(hash));
					},
					(
						SigningState::Commit { signature_shares, .. },
						SigningMessage::Sign { signature_share },
					) => {
						if signature_share.identifier != frost_id {
							self.report(frost_id);
							return;
						}
						signature_shares.insert(peer_id, signature_share);
					},
					(
						SigningState::Sign { signature_shares, .. },
						SigningMessage::Sign { signature_share },
					) => {
						if signature_share.identifier != frost_id {
							self.report(frost_id);
							return;
						}
						signature_shares.insert(peer_id, signature_share);
						self.transition(Some(hash));
					},
					(state, msg) => {
						log::error!(
							"invalid state ({}, {}, {}, {})",
							self.peer_id,
							state,
							peer_id,
							msg
						);
					},
				}
			},
			(state, msg) => {
				log::error!("invalid state ({}, {}, {}, {})", self.peer_id, state, peer_id, msg);
			},
		}
	}

	fn transition(&mut self, hash: Option<[u8; 32]>) {
		log::debug!("{} transition", self.peer_id);
		let mut step = true;
		while step {
			step = false;
			match (&mut self.state, hash) {
				(
					TssState::DkgR1 {
						secret_package,
						round1_packages,
						round2_packages,
					},
					None,
				) => {
					if round1_packages.len() == self.config.total_nodes - 1 {
						let round1_packages =
							std::mem::take(round1_packages).into_values().collect::<Vec<_>>();
						match dkg::part2(secret_package.take().unwrap(), &round1_packages) {
							Ok((secret_package, round2_package)) => {
								self.state = TssState::DkgR2 {
									secret_package,
									round1_packages,
									round2_packages: std::mem::take(round2_packages),
								};
								self.actions.push_back(TssAction::Timeout(Timeout::DKGR2));
								self.actions.push_back(TssAction::Send(TssMessage::DkgR2 {
									round2_package,
								}));
								step = true;
							},
							Err(Error::InvalidProofOfKnowledge { sender }) => {
								self.report(sender);
							},
							Err(err) => unreachable!("{err}"),
						}
					}
				},
				(
					TssState::DkgR2 {
						secret_package,
						round1_packages,
						round2_packages,
					},
					None,
				) => {
					if round2_packages.len() == self.config.total_nodes - 1 {
						let round2_packages =
							std::mem::take(round2_packages).into_values().collect::<Vec<_>>();
						match dkg::part3(secret_package, round1_packages, &round2_packages) {
							Ok((key_package, public_key_package)) => {
								let group_public =
									VerifyingKey::new(public_key_package.group_public);
								self.state = TssState::Initialized {
									key_package,
									public_key_package,
									signing_state: Default::default(),
								};
								self.actions.push_back(TssAction::PublicKey(group_public));
								step = true;
							},
							Err(Error::InvalidSecretShare { identifier }) => {
								self.report(identifier);
							},
							Err(err) => unreachable!("{err}"),
						}
					}
				},
				(
					TssState::Initialized {
						key_package,
						public_key_package,
						signing_state,
					},
					Some(hash),
				) => match signing_state.entry(hash).or_default() {
					SigningState::Commit {
						data,
						nonces,
						commitments,
						signature_shares,
					} => {
						if commitments.len() == self.config.total_nodes {
							let data = std::mem::take(data);
							let commitments =
								std::mem::take(commitments).into_values().collect::<Vec<_>>();
							let signing_package = SigningPackage::new(commitments, data);
							let signature_share =
								round2::sign(&signing_package, nonces, key_package)
									.expect("valid inputs");
							let mut signature_shares = std::mem::take(signature_shares);
							signature_shares.insert(self.peer_id.clone(), signature_share);
							let msg = SigningMessage::Sign { signature_share };
							signing_state.insert(
								hash,
								SigningState::Sign {
									signing_package,
									signature_shares,
								},
							);
							self.actions.push_back(TssAction::Timeout(Timeout::sign(hash)));
							self.actions.push_back(TssAction::Send(TssMessage::Sign { hash, msg }));
							step = true;
						}
					},
					SigningState::Sign {
						signing_package,
						signature_shares,
					} => {
						if signature_shares.len() == self.config.total_nodes {
							let shares =
								std::mem::take(signature_shares).into_values().collect::<Vec<_>>();
							match frost_evm::aggregate(signing_package, &shares, public_key_package)
							{
								Ok(signature) => {
									self.actions.push_back(TssAction::Tss(signature));
									signing_state.remove(&hash);
									step = true;
								},
								Err(Error::InvalidSignatureShare { signer }) => {
									self.report(signer);
								},
								Err(err) => unreachable!("{err}"),
							}
						}
					},
					_ => {},
				},
				_ => {},
			}
		}
	}

	pub fn sign(&mut self, data: Vec<u8>) {
		let hash = blake3::hash(&data).into();
		log::debug!("{} sign", self.peer_id);
		match &mut self.state {
			TssState::Initialized { key_package, signing_state, .. } => {
				let mut commitments = match signing_state.entry(hash).or_default() {
					SigningState::PreCommit { commitments } => std::mem::take(commitments),
					_ => return,
				};
				let (nonces, commitment) =
					round1::commit(self.frost_id, key_package.secret_share(), &mut OsRng);
				commitments.insert(self.peer_id.clone(), commitment);
				signing_state.insert(
					hash,
					SigningState::Commit {
						data,
						nonces,
						commitments,
						signature_shares: Default::default(),
					},
				);
				self.actions.push_back(TssAction::Timeout(Timeout::commit(hash)));
				let msg = SigningMessage::Commit { commitment };
				self.actions.push_back(TssAction::Send(TssMessage::Sign { hash, msg }));
			},
			_ => panic!("invalid state"),
		}
		self.transition(Some(hash));
	}

	pub fn next_action(&mut self) -> Option<TssAction<P>> {
		self.actions.pop_front()
	}
}

impl<P: std::fmt::Display> std::fmt::Display for Tss<P> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{} {}", self.peer_id, self.state)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Default)]
	struct TssEvents {
		pubkeys: BTreeMap<u8, VerifyingKey>,
		signatures: BTreeMap<u8, Signature>,
		reports: BTreeMap<u8, u8>,
		timeouts: BTreeMap<u8, Option<Timeout>>,
	}

	impl TssEvents {
		fn assert_pubkeys(&self, n: usize) -> VerifyingKey {
			assert_eq!(self.pubkeys.len(), n);
			let first = self.pubkeys.values().next().unwrap();
			for pubkey in self.pubkeys.values() {
				assert_eq!(pubkey, first);
			}
			first.clone()
		}

		fn assert_signatures(&self, n: usize) -> Signature {
			assert_eq!(self.signatures.len(), n);
			let first = self.signatures.values().next().unwrap();
			for sig in self.signatures.values() {
				assert_eq!(sig, first);
			}
			first.clone()
		}

		fn assert_reports(&self, n: usize) -> u8 {
			let mut score = BTreeMap::new();
			for (reporter, offender) in &self.reports {
				if reporter == offender {
					continue;
				}
				*score.entry(offender).or_insert(0) += 1;
			}
			let mut inv_score: BTreeMap<_, _> = score.into_iter().map(|(k, v)| (v, k)).collect();
			let (score, offender) = inv_score.pop_last().unwrap();
			assert_eq!(score, n);
			*offender
		}

		fn assert_timeouts(&self, n: usize) {
			let timeouts =
				self.timeouts.values().filter_map(|timeout| *timeout).collect::<Vec<_>>();
			assert_eq!(timeouts.len(), n);
		}

		fn assert(&self, pubkeys: usize, signatures: usize, reports: usize, timeouts: usize) {
			if pubkeys == 0 {
				assert!(self.pubkeys.is_empty());
			} else {
				self.assert_pubkeys(pubkeys);
			}
			if signatures == 0 {
				assert!(self.signatures.is_empty());
			} else {
				self.assert_signatures(signatures);
			}
			if reports == 0 {
				assert!(self.reports.is_empty());
			} else {
				self.assert_reports(reports);
			}
			self.assert_timeouts(timeouts);
		}
	}

	type FaultInjector = Box<dyn FnMut(u8, TssMessage) -> Option<TssMessage>>;

	struct TssTester {
		tss: Vec<Tss<u8>>,
		actions: VecDeque<(u8, TssAction<u8>)>,
		events: TssEvents,
		fault_injector: FaultInjector,
	}

	impl TssTester {
		pub fn new(n: usize) -> Self {
			Self::new_with_fault_injector(n, Box::new(|_, msg| Some(msg)))
		}

		pub fn new_with_fault_injector(n: usize, fault_injector: FaultInjector) -> Self {
			let mut tss = Vec::with_capacity(n);
			for i in 0..n {
				tss.push(Tss::new(i as u8));
			}
			Self {
				tss,
				actions: Default::default(),
				events: Default::default(),
				fault_injector,
			}
		}

		pub fn initialize_one(&mut self, i: usize) {
			let n = self.tss.len();
			let members = (0..n).map(|i| i as u8).collect::<BTreeSet<_>>();
			let tss = &mut self.tss[i];
			tss.initialize(members, n as _);
			while let Some(action) = tss.next_action() {
				self.actions.push_back((*tss.peer_id(), action));
			}
		}

		pub fn initialize(&mut self) {
			let n = self.tss.len();
			let members = (0..n).map(|i| i as u8).collect::<BTreeSet<_>>();
			for tss in &mut self.tss {
				tss.initialize(members.clone(), n as _);
				while let Some(action) = tss.next_action() {
					self.actions.push_back((*tss.peer_id(), action));
				}
			}
		}

		pub fn sign_one(&mut self, i: usize, data: &[u8]) {
			let tss = &mut self.tss[i];
			tss.sign(data.to_vec());
			while let Some(action) = tss.next_action() {
				self.actions.push_back((*tss.peer_id(), action));
			}
		}

		pub fn sign(&mut self, data: &[u8]) {
			for tss in &mut self.tss {
				tss.sign(data.to_vec());
				while let Some(action) = tss.next_action() {
					self.actions.push_back((*tss.peer_id(), action));
				}
			}
		}

		pub fn timeout(&mut self, timeout: Timeout) {
			for tss in &mut self.tss {
				tss.on_timeout(timeout);
				while let Some(action) = tss.next_action() {
					self.actions.push_back((*tss.peer_id(), action));
				}
			}
		}

		pub fn run(&mut self) -> TssEvents {
			while let Some((peer_id, action)) = self.actions.pop_front() {
				match action {
					TssAction::Send(msg) => {
						if let Some(msg) = (self.fault_injector)(peer_id, msg) {
							for tss in &mut self.tss {
								tss.on_message(peer_id, msg.clone());
								while let Some(action) = tss.next_action() {
									self.actions.push_back((*tss.peer_id(), action));
								}
							}
						}
					},
					TssAction::Report(offender) => {
						assert!(self.events.reports.insert(peer_id, offender).is_none());
						self.events.timeouts.insert(peer_id, None);
					},
					TssAction::PublicKey(pubkey) => {
						assert!(self.events.pubkeys.insert(peer_id, pubkey).is_none());
						self.events.timeouts.insert(peer_id, None);
					},
					TssAction::Tss(sig) => {
						assert!(self.events.signatures.insert(peer_id, sig).is_none());
						self.events.timeouts.insert(peer_id, None);
					},
					TssAction::Timeout(timeout) => {
						self.events.timeouts.insert(peer_id, Some(timeout));
					},
				}
			}
			std::mem::take(&mut self.events)
		}
	}

	#[test]
	fn test_basic() {
		env_logger::try_init().ok();
		let n = 3;
		let mut tester = TssTester::new(n);
		tester.initialize();
		tester.run().assert(n, 0, 0, 0);
		tester.sign(b"a message");
		tester.run().assert(0, n, 0, 0);
	}

	#[test]
	fn test_fault_dkg() {
		env_logger::try_init().ok();
		let n = 5;
		let mut tester = TssTester::new_with_fault_injector(
			n,
			Box::new(|peer_id, mut msg| {
				if peer_id == 0 {
					if let TssMessage::DkgR2 { round2_package } = &mut msg {
						round2_package.pop();
					}
				}
				Some(msg)
			}),
		);
		tester.initialize();
		let offender = tester.run().assert_reports(n - 1);
		assert_eq!(offender, 0);
	}

	#[test]
	fn test_fault_sign() {
		env_logger::try_init().ok();
		let n = 5;
		let mut tester = TssTester::new_with_fault_injector(
			n,
			Box::new(|peer_id, mut msg| {
				if peer_id == 0 {
					if let TssMessage::Sign {
						msg: SigningMessage::Sign { signature_share },
						..
					} = &mut msg
					{
						signature_share.identifier = Identifier::try_from(2).unwrap();
					}
				}
				Some(msg)
			}),
		);
		tester.initialize();
		tester.run().assert(n, 0, 0, 0);
		tester.sign(b"message");
		let offender = tester.run().assert_reports(n - 1);
		assert_eq!(offender, 0);
	}

	#[test]
	fn test_initialize_race() {
		env_logger::try_init().ok();
		let n = 2;
		let mut tester = TssTester::new(n);
		tester.initialize_one(0);
		tester.run().assert(0, 0, 0, 2);
		tester.initialize_one(1);
		tester.run().assert(n, 0, 0, 0);
	}

	#[test]
	fn test_sign_race() {
		env_logger::try_init().ok();
		let n = 2;
		let mut tester = TssTester::new(n);
		tester.initialize();
		tester.run().assert(n, 0, 0, 0);
		tester.sign_one(0, b"a message");
		tester.run().assert(0, 0, 0, 2);
		tester.sign_one(1, b"a message");
		tester.run().assert(0, n, 0, 0);
	}

	#[test]
	fn test_timeout() {
		env_logger::try_init().ok();
		let n = 3;
		let mut tester = TssTester::new_with_fault_injector(
			n,
			Box::new(|peer_id, msg| if peer_id == 0 { None } else { Some(msg) }),
		);
		tester.initialize();
		tester.run().assert(0, 0, 0, 3);
		tester.timeout(Timeout::DKGR1);
		tester.run().assert(0, 0, n - 1, 0);
	}
}
