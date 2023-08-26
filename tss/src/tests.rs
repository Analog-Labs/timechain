use crate::roast::RoastMessage;
use crate::{Tss, TssAction, TssMessage};
use frost_evm::round2::SignatureShare;
use frost_evm::{Signature, VerifyingKey};
use std::collections::{BTreeMap, BTreeSet};

type Peer = u8;
type Id = u8;

#[derive(Default)]
struct TssEvents {
	pubkeys: BTreeMap<Peer, VerifyingKey>,
	signatures: BTreeMap<Peer, Signature>,
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

	fn assert_signatures(&self, n: usize) -> Option<Signature> {
		assert_eq!(self.signatures.len(), n);
		let first = self.signatures.values().next()?;
		for sig in self.signatures.values() {
			assert_eq!(sig, first);
		}
		Some(first.clone())
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
			for i in 0..self.tss.len() {
				let peer_id = *self.tss[i].peer_id();
				while let Some(action) = self.tss[i].next_action() {
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
						TssAction::Signature(_, sig) => {
							log::info!("{} action signature", peer_id);
							assert!(self.events.signatures.insert(peer_id, sig).is_none());
						},
					}
				}
			}
			if messages.is_empty() {
				break;
			}
			for (rx, tx, msg) in messages {
				log::info!("{} action send {} to {}", tx, msg, rx);
				self.tss[rx as usize].on_message(tx, msg);
			}
		}
		std::mem::take(&mut self.events)
	}
}

#[test]
fn test_basic() {
	env_logger::try_init().ok();
	let n = 5;
	let t = 3;
	let mut tester = TssTester::new(n, t);
	tester.run().assert_pubkeys(n);
	tester.sign(0, b"a message");
	tester.run().assert_signatures(t);
}

#[test]
fn test_fault_sign() {
	env_logger::try_init().ok();
	let n = 5;
	let t = 3;
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
	tester.run().assert_pubkeys(n);
	tester.sign(0, b"message");
	tester.run().assert_signatures(t);
}
