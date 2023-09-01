use crate::dkg::DkgMessage;
use crate::roast::RoastMessage;
use crate::{Tss, TssAction, TssMessage};
use frost_evm::keys::SigningShare;
use frost_evm::round2::SignatureShare;
use frost_evm::{Signature, VerifyingKey};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Peer = u8;
type Id = u8;

#[derive(Default)]
struct TssEvents {
	pubkeys: BTreeMap<Peer, VerifyingKey>,
	signatures: BTreeMap<Peer, Signature>,
	reports: BTreeMap<Peer, Option<Peer>>,
}

impl TssEvents {
	fn assert_pubkeys(&self, n: usize) -> VerifyingKey {
		assert_eq!(self.pubkeys.len(), n);
		let first = self.pubkeys.values().next().unwrap();
		for pubkey in self.pubkeys.values() {
			assert_eq!(pubkey, first);
		}
		*first
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

	fn assert(&self, pubkeys: usize, signatures: usize, reports: usize) {
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
		let members = (0..n).map(|i| i as _).collect::<BTreeSet<_>>();
		let mut tss = Vec::with_capacity(n);
		let mut actions = VecDeque::default();
		for i in 0..n {
			let mut tssi = Tss::new(i as _, members.clone(), n as _);
			while let Some(action) = tssi.next_action() {
				actions.push_back((*tssi.peer_id(), action));
			}
			tss.push(tssi);
		}
		Self {
			tss,
			actions,
			events: Default::default(),
			fault_injector,
		}
	}

	#[allow(unused)]
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

	pub fn run(&mut self) -> TssEvents {
		while let Some((peer_id, action)) = self.actions.pop_front() {
			match action {
				TssAction::Send(msgs) => {
					for (peer, msg) in msgs {
						if let Some(msg) = (self.fault_injector)(peer_id, msg) {
							let tss = &mut self.tss[usize::from(peer)];
							tss.on_message(peer_id, msg.clone());
							while let Some(action) = tss.next_action() {
								self.actions.push_back((*tss.peer_id(), action));
							}
						}
					}
				},
				TssAction::PublicKey(pubkey) => {
					assert!(self.events.pubkeys.insert(peer_id, pubkey).is_none());
				},
				TssAction::Signature(_, _, sig) => {
					assert!(self.events.signatures.insert(peer_id, sig).is_none());
				},
				TssAction::Error(_, offender, _) => {
					assert!(self.events.reports.insert(peer_id, offender).is_none());
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
	tester.run().assert(n, 0, 0);
	tester.sign(0, b"a message");
	tester.run().assert(0, n, 0);
}

#[test]
fn test_fault_dkg() {
	env_logger::try_init().ok();
	let n = 5;
	let mut tester = TssTester::new_with_fault_injector(
		n,
		Box::new(|peer_id, msg| {
			if peer_id == 0 {
				if let TssMessage::Dkg { msg: DkgMessage::DkgR2 { .. } } = msg {
					let round2_package = frost_evm::keys::dkg::round2::Package::new(
						SigningShare::deserialize([42; 32]).unwrap(),
					);
					return Some(TssMessage::Dkg {
						msg: DkgMessage::DkgR2 { round2_package },
					});
				}
			}
			Some(msg)
		}),
	);
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
				if let TssMessage::Roast {
					msg: RoastMessage::Sign { signature_share },
					..
				} = &mut msg
				{
					*signature_share = SignatureShare::deserialize([42; 32]).unwrap();
				}
			}
			Some(msg)
		}),
	);
	tester.run().assert(n, 0, 0);
	tester.sign(0, b"message");
	let offender = tester.run().assert_reports(n - 1);
	assert_eq!(offender, Some(0));
}
