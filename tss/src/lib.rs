#![allow(clippy::large_enum_variant)]
use frost_evm::{
	keys::{dkg, KeyPackage, PublicKeyPackage},
	round1::{self, SigningCommitments, SigningNonces},
	round2::{self, SignatureShare},
	Error, Identifier, Signature, SigningPackage, VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

/// Tss state.
enum TssState<I, P> {
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
		round1_packages: HashMap<Identifier, dkg::round1::Package>,
		round2_packages: BTreeMap<P, dkg::round2::Package>,
	},
	Initialized {
		key_package: KeyPackage,
		public_key_package: PublicKeyPackage,
		signing_state: BTreeMap<I, SigningState<P>>,
	},
}

impl<I, P> Default for TssState<I, P> {
	fn default() -> Self {
		Self::Uninitialized {
			round1_packages: Default::default(),
		}
	}
}

impl<I, P> std::fmt::Display for TssState<I, P> {
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
pub enum TssAction<I, P> {
	Send(P, TssMessage<I>),
	PublicKey(VerifyingKey),
	Tss(Signature, I),
	Report(Option<P>, Option<I>),
	Timeout(Timeout<I>, Option<I>),
}

/// Tss message.
#[derive(Clone, Deserialize, Serialize)]
pub enum TssMessage<I> {
	DkgR1 { round1_package: dkg::round1::Package },
	DkgR2 { round2_package: dkg::round2::Package },
	Sign { id: I, msg: SigningMessage },
}

impl<I> std::fmt::Display for TssMessage<I> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::DkgR1 { .. } => write!(f, "dkgr1"),
			Self::DkgR2 { .. } => write!(f, "dkgr2"),
			Self::Sign { msg, .. } => msg.fmt(f),
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
pub struct Timeout<I>(TimeoutKind<I>);

impl<I> Timeout<I> {
	const UNINITIALIZED: Self = Self(TimeoutKind::Uninitialized);
	const DKGR1: Self = Self(TimeoutKind::DkgR1);
	const DKGR2: Self = Self(TimeoutKind::DkgR2);
	const fn precommit(id: I) -> Self {
		Self(TimeoutKind::Sign(id, SignTimeoutKind::PreCommit))
	}
	const fn commit(id: I) -> Self {
		Self(TimeoutKind::Sign(id, SignTimeoutKind::Commit))
	}
	const fn sign(id: I) -> Self {
		Self(TimeoutKind::Sign(id, SignTimeoutKind::Sign))
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimeoutKind<I> {
	Uninitialized,
	DkgR1,
	DkgR2,
	Sign(I, SignTimeoutKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignTimeoutKind {
	PreCommit,
	Commit,
	Sign,
}

fn peer_to_frost(peer: impl std::fmt::Display) -> Identifier {
	Identifier::derive(peer.to_string().as_bytes()).expect("non zero")
}

struct TssConfig<P> {
	frost_to_peer: BTreeMap<Identifier, P>,
	threshold: usize,
	total_nodes: usize,
}

impl<P> Default for TssConfig<P> {
	fn default() -> Self {
		Self {
			frost_to_peer: Default::default(),
			threshold: 0,
			total_nodes: 0,
		}
	}
}

impl<P: Clone + Ord + std::fmt::Display> TssConfig<P> {
	pub fn new(peer_id: &P, members: BTreeSet<P>, threshold: u16) -> Self {
		debug_assert!(members.contains(peer_id));
		let threshold = threshold as _;
		let total_nodes = members.len();
		let frost_to_peer = members.into_iter().map(|peer| (peer_to_frost(&peer), peer)).collect();
		Self {
			frost_to_peer,
			threshold,
			total_nodes,
		}
	}
}

/// Tss state machine.
pub struct Tss<I, P> {
	peer_id: P,
	frost_id: Identifier,
	config: TssConfig<P>,
	state: TssState<I, P>,
	actions: VecDeque<TssAction<I, P>>,
}

impl<I, P> Tss<I, P>
where
	I: Clone + Copy + Ord + std::fmt::Debug,
	P: Clone + Ord + std::fmt::Display,
{
	pub fn new(peer_id: P) -> Self {
		let frost_id = peer_to_frost(&peer_id);
		Self {
			peer_id,
			frost_id,
			config: Default::default(),
			state: Default::default(),
			actions: Default::default(),
		}
	}

	pub fn peer_id(&self) -> &P {
		&self.peer_id
	}

	pub fn is_initialized(&self) -> bool {
		!matches!(&self.state, TssState::Uninitialized { .. })
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

	fn report(&mut self, frost: Option<Identifier>, id: Option<I>) {
		if let (TssState::Initialized { signing_state, .. }, Some(id)) = (&mut self.state, id) {
			signing_state.remove(&id);
		} else {
			self.state = TssState::default();
		}
		let peer = frost.map(|frost| self.frost_to_peer(&frost));
		self.actions.push_back(TssAction::Report(peer, id));
	}

	fn broadcast(&mut self, msg: &TssMessage<I>) {
		for peer in self.config.frost_to_peer.values() {
			if peer == &self.peer_id {
				continue;
			}
			self.actions.push_back(TssAction::Send(peer.clone(), msg.clone()));
		}
	}

	pub fn initialize(&mut self, members: BTreeSet<P>, threshold: u16) {
		log::debug!("{} initialize {}/{}", self.peer_id, threshold, members.len());
		let round1_packages_preinit = match &mut self.state {
			TssState::Uninitialized { round1_packages } => std::mem::take(round1_packages),
			state => panic!("invalid state ({state})"),
		};
		self.config = TssConfig::new(&self.peer_id, members, threshold);
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
		self.actions.push_back(TssAction::Timeout(Timeout::DKGR1, None));
		self.broadcast(&TssMessage::DkgR1 { round1_package });
		for (peer_id, round1_package) in round1_packages_preinit {
			self.on_message(peer_id, TssMessage::DkgR1 { round1_package });
		}
	}

	pub fn on_timeout(&mut self, timeout: Timeout<I>) {
		log::debug!("{} on timeout {:?}", self.peer_id, timeout);
		let mut report = vec![];
		let mut timeout_id = None;
		match (&self.state, timeout.0) {
			(TssState::Uninitialized { round1_packages }, TimeoutKind::Uninitialized) => {
				for (frost_id, peer_id) in &self.config.frost_to_peer {
					if round1_packages.contains_key(peer_id) {
						report.push(*frost_id);
					}
				}
			},
			(TssState::DkgR1 { round1_packages, .. }, TimeoutKind::DkgR1) => {
				for (frost_id, peer_id) in &self.config.frost_to_peer {
					if peer_id == &self.peer_id {
						continue;
					}
					if !round1_packages.contains_key(peer_id) {
						report.push(*frost_id);
					}
				}
			},
			(TssState::DkgR2 { round2_packages, .. }, TimeoutKind::DkgR2) => {
				for (frost_id, peer_id) in &self.config.frost_to_peer {
					if peer_id == &self.peer_id {
						continue;
					}
					if !round2_packages.contains_key(peer_id) {
						report.push(*frost_id);
					}
				}
			},
			(TssState::Initialized { signing_state, .. }, TimeoutKind::Sign(id, timeout)) => {
				timeout_id = Some(id);
				match (signing_state.get(&id), timeout) {
					(Some(SigningState::PreCommit { commitments }), SignTimeoutKind::PreCommit) => {
						for (frost_id, peer_id) in &self.config.frost_to_peer {
							if commitments.contains_key(peer_id) {
								report.push(*frost_id);
							}
						}
					},
					(Some(SigningState::Commit { commitments, .. }), SignTimeoutKind::Commit) => {
						for (frost_id, peer_id) in &self.config.frost_to_peer {
							if !commitments.contains_key(peer_id) {
								report.push(*frost_id);
							}
						}
					},
					(Some(SigningState::Sign { signature_shares, .. }), SignTimeoutKind::Sign) => {
						for (frost_id, peer_id) in &self.config.frost_to_peer {
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
			self.report(Some(report), timeout_id);
		}
	}

	pub fn on_message(&mut self, peer_id: P, msg: TssMessage<I>) {
		log::debug!("{} on_message {} {}", self.peer_id, peer_id, msg);
		if self.peer_id == peer_id {
			log::debug!("{} dropping message from self", self.peer_id);
			return;
		}
		if self.config.frost_to_peer.is_empty() {
			match (&mut self.state, msg) {
				(
					TssState::Uninitialized { round1_packages },
					TssMessage::DkgR1 { round1_package },
				) => {
					log::debug!("{} inserted_round1 package", self.peer_id);
					round1_packages.insert(peer_id, round1_package);
					self.actions.push_back(TssAction::Timeout(Timeout::UNINITIALIZED, None));
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
		let frost_id = peer_to_frost(&peer_id);
		if !self.config.frost_to_peer.contains_key(&frost_id) {
			log::info!("{} dropping message from {}", self.peer_id, peer_id);
			return;
		}
		log::debug!("on_message processing started");
		match (&mut self.state, msg) {
			(TssState::DkgR1 { round1_packages, .. }, TssMessage::DkgR1 { round1_package, .. }) => {
				round1_packages.insert(peer_id, round1_package);
				self.transition(None);
			},
			(TssState::DkgR1 { round2_packages, .. }, TssMessage::DkgR2 { round2_package, .. }) => {
				round2_packages.insert(peer_id, round2_package);
			},
			(TssState::DkgR2 { round2_packages, .. }, TssMessage::DkgR2 { round2_package, .. }) => {
				round2_packages.insert(peer_id, round2_package);
				self.transition(None);
			},
			(TssState::Initialized { signing_state, .. }, TssMessage::Sign { id, msg }) => {
				if !signing_state.contains_key(&id) {
					self.actions.push_back(TssAction::Timeout(Timeout::precommit(id), Some(id)));
				}
				let state = signing_state.entry(id).or_default();
				match (state, msg) {
					(
						SigningState::PreCommit { commitments },
						SigningMessage::Commit { commitment },
					) => {
						commitments.insert(peer_id, commitment);
					},
					(
						SigningState::Commit { commitments, .. },
						SigningMessage::Commit { commitment },
					) => {
						commitments.insert(peer_id, commitment);
						self.transition(Some(id));
					},
					(
						SigningState::Commit { signature_shares, .. },
						SigningMessage::Sign { signature_share },
					) => {
						signature_shares.insert(peer_id, signature_share);
					},
					(
						SigningState::Sign { signature_shares, .. },
						SigningMessage::Sign { signature_share },
					) => {
						signature_shares.insert(peer_id, signature_share);
						self.transition(Some(id));
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

	fn transition(&mut self, id: Option<I>) {
		log::debug!("{} transition", self.peer_id);
		let mut step = true;
		while step {
			step = false;
			match (&mut self.state, id) {
				(
					TssState::DkgR1 {
						secret_package,
						round1_packages,
						round2_packages,
					},
					None,
				) => {
					if round1_packages.len() == self.config.total_nodes - 1 {
						log::debug!("received all packages for dk2 processing transition");
						let round1_packages = std::mem::take(round1_packages)
							.into_iter()
							.map(|(peer, package)| (peer_to_frost(&peer), package))
							.collect();
						match dkg::part2(secret_package.take().unwrap(), &round1_packages) {
							Ok((secret_package, round2_package)) => {
								self.state = TssState::DkgR2 {
									secret_package,
									round1_packages,
									round2_packages: std::mem::take(round2_packages),
								};
								self.actions.push_back(TssAction::Timeout(Timeout::DKGR2, None));
								for (identifier, package) in round2_package {
									let peer = self.frost_to_peer(&identifier);
									self.actions.push_back(TssAction::Send(
										peer,
										TssMessage::DkgR2 { round2_package: package },
									));
								}
								step = true;
							},
							Err(Error::InvalidProofOfKnowledge { culprit }) => {
								self.report(Some(culprit), None);
							},
							Err(err) => unreachable!("{err}"),
						}
					} else {
						log::debug!(
							"transition to dkg2 need packages {} got {}",
							round1_packages.len(),
							self.config.total_nodes - 1
						);
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
						log::debug!("received all packages for dk3 processing transition");
						let round2_packages = std::mem::take(round2_packages)
							.into_iter()
							.map(|(peer, package)| (peer_to_frost(&peer), package))
							.collect();
						match dkg::part3(secret_package, round1_packages, &round2_packages) {
							Ok((key_package, public_key_package)) => {
								let group_public =
									VerifyingKey::new(*public_key_package.group_public());
								self.state = TssState::Initialized {
									key_package,
									public_key_package,
									signing_state: Default::default(),
								};
								self.actions.push_back(TssAction::PublicKey(group_public));
								step = true;
							},
							Err(Error::InvalidSecretShare) => {
								self.report(None, None);
							},
							Err(err) => unreachable!("{err}"),
						}
					} else {
						log::debug!(
							"transition to dkg3 need packages {} got {}",
							round2_packages.len(),
							self.config.total_nodes - 1
						);
					}
				},
				(
					TssState::Initialized {
						key_package,
						public_key_package,
						signing_state,
					},
					Some(id),
				) => match signing_state.entry(id).or_default() {
					SigningState::Commit {
						data,
						nonces,
						commitments,
						signature_shares,
					} => {
						if commitments.len() == self.config.total_nodes {
							log::debug!("received all commitments processing signing");
							let data = std::mem::take(data);
							let commitments = std::mem::take(commitments)
								.into_iter()
								.map(|(peer, package)| (peer_to_frost(&peer), package))
								.collect();
							let signing_package = SigningPackage::new(commitments, &data);
							let signature_share =
								round2::sign(&signing_package, nonces, key_package)
									.expect("valid inputs");
							let mut signature_shares = std::mem::take(signature_shares);
							signature_shares.insert(self.peer_id.clone(), signature_share);
							let msg = SigningMessage::Sign { signature_share };
							signing_state.insert(
								id,
								SigningState::Sign {
									signing_package,
									signature_shares,
								},
							);
							self.actions.push_back(TssAction::Timeout(Timeout::sign(id), Some(id)));
							self.broadcast(&TssMessage::Sign { id, msg: msg.clone() });
							step = true;
						} else {
							log::debug!(
								"commitments needed {} got {}",
								self.config.total_nodes,
								commitments.len()
							);
						}
					},
					SigningState::Sign {
						signing_package,
						signature_shares,
					} => {
						if signature_shares.len() == self.config.total_nodes {
							log::debug!("Received all shares processing aggregator");
							let shares = std::mem::take(signature_shares)
								.into_iter()
								.map(|(peer, share)| (peer_to_frost(&peer), share))
								.collect();
							match frost_evm::aggregate(signing_package, &shares, public_key_package)
							{
								Ok(signature) => {
									log::info!("Aggregated signature successfully");
									self.actions.push_back(TssAction::Tss(signature, id));
									signing_state.remove(&id);
									step = true;
								},
								Err(Error::InvalidSignatureShare { culprit }) => {
									self.report(Some(culprit), Some(id));
								},
								Err(err) => unreachable!("{err}"),
							}
						} else {
							log::debug!(
								"Required signature shares are {} got {}",
								self.config.total_nodes,
								signature_shares.len()
							);
						}
					},
					_ => {},
				},
				_ => {},
			}
		}
	}

	pub fn sign(&mut self, id: I, data: Vec<u8>) {
		log::debug!("{} sign", self.peer_id);
		match &mut self.state {
			TssState::Initialized { key_package, signing_state, .. } => {
				let mut commitments = match signing_state.entry(id).or_default() {
					SigningState::PreCommit { commitments } => std::mem::take(commitments),
					_ => {
						log::warn!("Signing already in progress for hash {:?}", data);
						return;
					},
				};
				let (nonces, commitment) = round1::commit(key_package.secret_share(), &mut OsRng);
				commitments.insert(self.peer_id.clone(), commitment);
				signing_state.insert(
					id,
					SigningState::Commit {
						data,
						nonces,
						commitments,
						signature_shares: Default::default(),
					},
				);
				self.actions.push_back(TssAction::Timeout(Timeout::commit(id), Some(id)));
				let msg = SigningMessage::Commit { commitment };
				self.broadcast(&TssMessage::Sign { id, msg });
			},
			_ => panic!("invalid state"),
		}
		self.transition(Some(id));
	}

	pub fn next_action(&mut self) -> Option<TssAction<I, P>> {
		self.actions.pop_front()
	}
}

impl<I, P: std::fmt::Display> std::fmt::Display for Tss<I, P> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{} {}", self.peer_id, self.state)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frost_evm::keys::SigningShare;

	type Peer = u8;
	type Id = u8;

	#[derive(Default)]
	struct TssEvents {
		pubkeys: BTreeMap<Peer, VerifyingKey>,
		signatures: BTreeMap<Peer, Signature>,
		reports: BTreeMap<Peer, Option<Peer>>,
		timeouts: BTreeMap<Peer, Option<Timeout<Id>>>,
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

		fn assert_reports(&self, n: usize) -> Option<Peer> {
			let mut score = BTreeMap::new();
			for (reporter, offender) in &self.reports {
				if Some(*reporter) == *offender {
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

	type FaultInjector = Box<dyn FnMut(Peer, TssMessage<Id>) -> Option<TssMessage<Id>>>;

	struct TssTester {
		tss: Vec<Tss<Id, Peer>>,
		actions: VecDeque<(Peer, TssAction<Id, Peer>)>,
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
				tss.push(Tss::new(i as _));
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
			let members = (0..n).map(|i| i as _).collect::<BTreeSet<_>>();
			let tss = &mut self.tss[i];
			tss.initialize(members, n as _);
			while let Some(action) = tss.next_action() {
				self.actions.push_back((*tss.peer_id(), action));
			}
		}

		pub fn initialize(&mut self) {
			let n = self.tss.len();
			let members = (0..n).map(|i| i as _).collect::<BTreeSet<_>>();
			for tss in &mut self.tss {
				tss.initialize(members.clone(), n as _);
				while let Some(action) = tss.next_action() {
					self.actions.push_back((*tss.peer_id(), action));
				}
			}
		}

		pub fn sign_one(&mut self, peer: usize, id: Id, data: &[u8]) {
			let tss = &mut self.tss[peer];
			tss.sign(id, data.to_vec());
			while let Some(action) = tss.next_action() {
				self.actions.push_back((*tss.peer_id(), action));
			}
		}

		pub fn sign(&mut self, id: u8, data: &[u8]) {
			for tss in &mut self.tss {
				tss.sign(id, data.to_vec());
				while let Some(action) = tss.next_action() {
					self.actions.push_back((*tss.peer_id(), action));
				}
			}
		}

		pub fn timeout(&mut self, timeout: Timeout<u8>) {
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
					TssAction::Send(peer, msg) => {
						if let Some(msg) = (self.fault_injector)(peer_id, msg) {
							let tss = &mut self.tss[usize::from(peer)];
							tss.on_message(peer_id, msg.clone());
							while let Some(action) = tss.next_action() {
								self.actions.push_back((*tss.peer_id(), action));
							}
						}
					},
					TssAction::Report(offender, _) => {
						assert!(self.events.reports.insert(peer_id, offender).is_none());
						self.events.timeouts.insert(peer_id, None);
					},
					TssAction::PublicKey(pubkey) => {
						assert!(self.events.pubkeys.insert(peer_id, pubkey).is_none());
						self.events.timeouts.insert(peer_id, None);
					},
					TssAction::Tss(sig, _) => {
						assert!(self.events.signatures.insert(peer_id, sig).is_none());
						self.events.timeouts.insert(peer_id, None);
					},
					TssAction::Timeout(timeout, _) => {
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
		tester.sign(0, b"a message");
		tester.run().assert(0, n, 0, 0);
	}

	#[test]
	fn test_fault_dkg() {
		env_logger::try_init().ok();
		let n = 5;
		let mut tester = TssTester::new_with_fault_injector(
			n,
			Box::new(|peer_id, msg| {
				if peer_id == 0 {
					if let TssMessage::DkgR2 { .. } = msg {
						let round2_package =
							dkg::round2::Package::new(SigningShare::deserialize([42; 32]).unwrap());
						return Some(TssMessage::DkgR2 { round2_package });
					}
				}
				Some(msg)
			}),
		);
		tester.initialize();
		let offender = tester.run().assert_reports(n - 1);
		assert_eq!(offender, None);
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
						*signature_share = SignatureShare::deserialize([42; 32]).unwrap();
					}
				}
				Some(msg)
			}),
		);
		tester.initialize();
		tester.run().assert(n, 0, 0, 0);
		tester.sign(0, b"message");
		let offender = tester.run().assert_reports(n - 1);
		assert_eq!(offender, Some(0));
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
		tester.sign_one(0, 0, b"a message");
		tester.run().assert(0, 0, 0, 2);
		tester.sign_one(1, 0, b"a message");
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
