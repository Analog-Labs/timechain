use crate::dkg::DkgMessage;
use crate::roast::RoastMessage;
use crate::{Tss, TssAction, TssMessage};
use frost_evm::keys::SigningShare;
use frost_evm::round2::SignatureShare;
use frost_evm::{Signature, VerifyingKey};
use std::collections::{BTreeMap, BTreeSet};

type Peer = u8;
type Id = u8;

#[derive(Default)]
struct TssEvents {
	pubkeys: BTreeMap<Peer, VerifyingKey>,
	signatures: BTreeMap<Id, BTreeMap<Peer, Signature>>,
}

impl TssEvents {
	fn assert_pubkeys(&self, n: usize) -> Option<VerifyingKey> {
		assert_eq!(self.pubkeys.len(), n);
		let first = self.pubkeys.values().next()?;
		for pubkey in self.pubkeys.values() {
			assert_eq!(pubkey, first);
		}
		Some(*first)
	}

	fn assert_signatures(&self, n: usize, pubkey: &VerifyingKey, id: Id, message: &[u8]) {
		let signatures = self.signatures.get(&id).unwrap();
		assert_eq!(signatures.len(), n);
		for sig in signatures.values() {
			pubkey.verify(message, sig).unwrap();
		}
	}
}

type FaultInjector = Box<dyn FnMut(Peer, TssMessage<Id>) -> Option<TssMessage<Id>>>;

struct TssTester {
	tss: Vec<Tss<Id, Peer>>,
	events: TssEvents,
	fault_injector: FaultInjector,
}

impl TssTester {
	pub fn new(n: usize, t: usize) -> Self {
		Self::new_with_fault_injector(n, t, Box::new(|_, msg| Some(msg)))
	}

	pub fn new_with_fault_injector(n: usize, t: usize, fault_injector: FaultInjector) -> Self {
		let members = (0..n).map(|i| i as _).collect::<BTreeSet<_>>();
		let mut tss = Vec::with_capacity(n);
		for i in 0..n {
			tss.push(Tss::new(i as _, members.clone(), t as _));
		}
		Self {
			tss,
			events: Default::default(),
			fault_injector,
		}
	}

	pub fn sign(&mut self, id: u8, data: &[u8]) {
		for tss in &mut self.tss {
			tss.sign(id, data.to_vec());
		}
	}

	pub fn run(&mut self) -> TssEvents {
		loop {
			let mut messages: Vec<(Peer, Peer, TssMessage<Id>)> = vec![];
			let mut progress = false;
			for i in 0..self.tss.len() {
				let peer_id = *self.tss[i].peer_id();
				while let Some(action) = self.tss[i].next_action() {
					progress = true;
					match action {
						TssAction::Send(msgs) => {
							for (peer, msg) in msgs {
								if let Some(msg) = (self.fault_injector)(peer_id, msg) {
									messages.push((peer, peer_id, msg));
								}
							}
						},
						TssAction::PublicKey(pubkey) => {
							log::info!("{} action pubkey", peer_id);
							assert!(self.events.pubkeys.insert(peer_id, pubkey).is_none());
						},
						TssAction::Signature(id, sig) => {
							log::info!("{} action {} signature", peer_id, id);
							assert!(self
								.events
								.signatures
								.entry(id)
								.or_default()
								.insert(peer_id, sig)
								.is_none());
						},
					}
				}
			}
			for (rx, tx, msg) in messages {
				log::info!("{} action send {} to {}", tx, msg, rx);
				self.tss[rx as usize].on_message(tx, msg);
			}
			if !progress {
				break;
			}
		}
		std::mem::take(&mut self.events)
	}
}

#[test]
fn test_basic() {
	env_logger::try_init().ok();
	let n = 3;
	let t = 3;
	let sigs = n - t + 1;
	let msg = b"a message";
	let mut tester = TssTester::new(n, t);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, msg);
	tester.run().assert_signatures(sigs, &pubkey, 0, msg);
}

#[test]
fn test_multiple_signing_sessions() {
	env_logger::try_init().ok();
	let n = 3;
	let t = 3;
	let sigs = n - t + 1;
	let msg_a = b"a message";
	let msg_b = b"another message";
	let mut tester = TssTester::new(n, t);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, msg_a);
	tester.sign(1, msg_b);
	let events = tester.run();
	events.assert_signatures(sigs, &pubkey, 0, msg_a);
	events.assert_signatures(sigs, &pubkey, 1, msg_b);
}

#[test]
fn test_threshold_sign() {
	env_logger::try_init().ok();
	let n = 3;
	let t = 2;
	let sigs = n - t + 1;
	let msg = b"a message";
	let mut tester = TssTester::new(n, t);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, msg);
	tester.run().assert_signatures(sigs, &pubkey, 0, msg);
}

#[test]
fn test_fault_dkg() {
	env_logger::try_init().ok();
	let n = 3;
	let t = 3;
	let mut tester = TssTester::new_with_fault_injector(
		n,
		t,
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
	// the only one succeeding in generating a pubkey would be peer 0
	tester.run().assert_pubkeys(1);
}

#[test]
fn test_fault_sign() {
	env_logger::try_init().ok();
	let n = 3;
	let t = 2;
	let sigs = n - t + 1;
	let msg = b"a message";
	let mut tester = TssTester::new_with_fault_injector(
		n,
		t,
		Box::new(|peer_id, mut msg| {
			if peer_id == 0 {
				if let TssMessage::Roast {
					msg: RoastMessage::Sign { signature_share, .. },
					..
				} = &mut msg
				{
					*signature_share = SignatureShare::deserialize([42; 32]).unwrap();
				}
			}
			Some(msg)
		}),
	);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, msg);
	tester.run().assert_signatures(sigs, &pubkey, 0, msg);
}
