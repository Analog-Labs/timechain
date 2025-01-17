use crate::{
	sum_commitments, verify_proof_of_knowledge, Identifier, ToFrostIdentifier, Tss, TssAction,
	TssMessage,
};
use frost_evm::{Signature, VerifyingKey};
use std::collections::{BTreeMap, BTreeSet};
use tracing::{Level, Span};

pub(crate) fn init_logger() {
	tracing_subscriber::fmt()
		.pretty()
		.with_ansi(false)
		.with_max_level(tracing::Level::DEBUG)
		.with_file(true)
		.with_line_number(true)
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.try_init()
		.ok();
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct Peer(u8);

impl ToFrostIdentifier for Peer {
	fn to_frost(&self) -> Identifier {
		Identifier::derive(&[self.0]).unwrap()
	}
}

impl std::fmt::Display for Peer {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

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

type FaultInjector = Box<dyn FnMut(Peer, Peer, TssMessage<Id>) -> Option<TssMessage<Id>>>;

struct TssTester {
	tss: Vec<Tss<Id, Peer>>,
	events: TssEvents,
	fault_injector: FaultInjector,
	span: Span,
}

impl TssTester {
	pub fn new(n: usize, t: usize) -> Self {
		Self::new_with_fault_injector(n, t, Box::new(|_, _, msg| Some(msg)))
	}

	pub fn new_with_fault_injector(n: usize, t: usize, fault_injector: FaultInjector) -> Self {
		let span = tracing::span!(Level::INFO, "shard", shard_id = 42);
		let members = (0..n).map(|i| Peer(i as _)).collect::<BTreeSet<_>>();
		let mut tss = Vec::with_capacity(n);
		for i in 0..n {
			tss.push(Tss::new(Peer(i as _), members.clone(), t as _, None, &span));
		}
		Self {
			tss,
			events: Default::default(),
			fault_injector,
			span,
		}
	}

	pub fn sign(&mut self, id: u8, data: &[u8]) {
		for tss in &mut self.tss {
			tss.on_start(id);
		}
		for tss in &mut self.tss {
			tss.on_sign(id, data.to_vec());
		}
	}

	pub fn run(&mut self) -> TssEvents {
		loop {
			let mut progress = false;
			let mut commitments = vec![];
			for i in 0..self.tss.len() {
				let from = *self.tss[i].peer_id();
				while let Some(action) = self.tss[i].next_action() {
					progress = true;
					match action {
						TssAction::Commit(commitment, proof_of_knowledge) => {
							verify_proof_of_knowledge(from, &commitment, proof_of_knowledge)
								.unwrap();
							commitments.push(commitment);
							if commitments.len() == self.tss.len() {
								let commitments = commitments.iter().collect::<Vec<_>>();
								let commitment = sum_commitments(&commitments).unwrap();
								for tss in &mut self.tss {
									tss.on_commit(commitment.clone());
								}
							}
						},
						TssAction::Send(msgs) => {
							for (to, msg) in msgs {
								if let Some(msg) = (self.fault_injector)(from, to, msg) {
									self.tss[to.0 as usize].on_message(from, msg);
								}
							}
						},
						TssAction::Ready(_, _, pubkey) => {
							tracing::info!(parent: &self.span, "{} action pubkey", from);
							assert!(self.events.pubkeys.insert(from, pubkey).is_none());
						},
						TssAction::Signature(id, _hash, sig) => {
							tracing::info!(parent: &self.span, "{} action {} signature", from, id);
							assert!(self
								.events
								.signatures
								.entry(id)
								.or_default()
								.insert(from, sig)
								.is_none());
						},
					}
				}
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
	init_logger();
	let n = 3;
	let t = 3;
	let sigs = n - t + 1;
	let msg = [0u8; 32];
	let mut tester = TssTester::new(n, t);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, &msg);
	tester.run().assert_signatures(sigs, &pubkey, 0, &msg);
	panic!();
}

#[test]
fn test_multiple_signing_sessions() {
	init_logger();
	let n = 3;
	let t = 3;
	let sigs = n - t + 1;
	let msg_a = [0u8; 32];
	let msg_b = [1u8; 32];
	let mut tester = TssTester::new(n, t);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, &msg_a);
	tester.sign(1, &msg_b);
	let events = tester.run();
	events.assert_signatures(sigs, &pubkey, 0, &msg_a);
	events.assert_signatures(sigs, &pubkey, 1, &msg_b);
}

#[test]
fn test_threshold_sign() {
	init_logger();
	let n = 3;
	let t = 2;
	let sigs = n - t + 1;
	let msg = [0u8; 32];
	let mut tester = TssTester::new(n, t);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, &msg);
	tester.run().assert_signatures(sigs, &pubkey, 0, &msg);
}

/*#[test]
fn test_fault_dkg() {
	env_logger::try_init().ok();
	let n = 3;
	let t = 3;
	let mut tester = TssTester::new_with_fault_injector(
		n,
		t,
		Box::new(|peer_id, msg| {
			if peer_id == 0 {
				if let TssRequest::Dkg { msg: DkgMessage::DkgR2 { .. } } = msg {
					let round2_package = frost_evm::keys::dkg::round2::Package::new(
						SigningShare::deserialize([42; 32]).unwrap(),
					);
					return Some(TssRequest::Dkg {
						msg: TssRequest::DkgR2 { round2_package },
					});
				}
			}
			Some(msg)
		}),
	);
	// the only one succeeding in generating a pubkey would be peer 0
	tester.run().assert_pubkeys(1);
}*/

/*#[test]
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
				if let TssRequest::Roast {
					msg: RoastRequest::Sign(request),
					..
				} = &mut msg
				{
					request.signature_share = SignatureShare::deserialize([42; 32]).unwrap();
				}
			}
			Some(msg)
		}),
	);
	let pubkey = tester.run().assert_pubkeys(n).unwrap();
	tester.sign(0, msg);
	tester.run().assert_signatures(sigs, &pubkey, 0, msg);
}*/
