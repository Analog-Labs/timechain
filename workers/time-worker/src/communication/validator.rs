use crate::TW_LOG;
use log::debug;
use parking_lot::{Mutex, RwLock};
use sc_network::PeerId;
use sc_network_gossip::{MessageIntent, ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header, NumberFor};
use std::{collections::BTreeMap, time::Duration};
use tokio::time::Instant;

const REBROADCAST_AFTER: Duration = Duration::from_secs(60 * 5);

#[allow(dead_code)]

pub type MessageHash = [u8; 8];

/// Gossip engine messages topic
pub(crate) fn topic<B: Block>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"/time/1")
}

struct KnownVotes<B: Block> {
	last_done: Option<NumberFor<B>>,
	live: BTreeMap<NumberFor<B>, fnv::FnvHashSet<MessageHash>>,
}

impl<B: Block> KnownVotes<B> {
	#[allow(dead_code)]
	pub fn new() -> Self {
		Self {
			last_done: None,
			live: BTreeMap::new(),
		}
	}

	#[allow(dead_code)]
	/// Create new round votes set if not already present.
	fn insert(&mut self, round: NumberFor<B>) {
		self.live.entry(round).or_default();
	}

	#[allow(dead_code)]
	/// Remove `round` and older from live set, update `last_done` accordingly.
	fn conclude(&mut self, round: NumberFor<B>) {
		self.live.retain(|&number, _| number > round);
		self.last_done = self.last_done.max(Some(round));
	}

	#[allow(dead_code)]
	/// Return true if `round` is newer than previously concluded rounds.
	///
	/// Latest concluded round is still considered alive to allow proper gossiping for it.
	fn is_live(&self, round: &NumberFor<B>) -> bool {
		Some(*round) >= self.last_done
	}

	#[allow(dead_code)]
	/// Add new _known_ `hash` to the round's known votes.
	fn add_known(&mut self, round: &NumberFor<B>, hash: MessageHash) {
		self.live.get_mut(round).map(|known| known.insert(hash));
	}

	#[allow(dead_code)]
	/// Check if `hash` is already part of round's known votes.
	fn is_known(&self, round: &NumberFor<B>, hash: &MessageHash) -> bool {
		self.live.get(round).map(|known| known.contains(hash)).unwrap_or(false)
	}
}

pub(crate) struct GossipValidator<B>
where
	B: Block,
{
	topic: B::Hash,
	known_votes: RwLock<KnownVotes<B>>,
	next_rebroadcast: Mutex<Instant>,
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	#[allow(dead_code)]
	pub fn new() -> GossipValidator<B> {
		GossipValidator {
			topic: topic::<B>(),
			known_votes: RwLock::new(KnownVotes::new()),
			next_rebroadcast: Mutex::new(Instant::now() + REBROADCAST_AFTER),
		}
	}

	#[allow(dead_code)]
	/// Note a voting round.
	///
	/// Noting round will start a live `round`.
	pub(crate) fn note_round(&self, round: NumberFor<B>) {
		debug!(target: TW_LOG, "About to note gossip round #{}", round);
		self.known_votes.write().insert(round);
	}

	#[allow(dead_code)]
	/// Conclude a voting round.
	///
	/// This can be called once round is complete so we stop gossiping for it.
	pub(crate) fn conclude_round(&self, round: NumberFor<B>) {
		debug!(target: TW_LOG, "About to drop gossip round #{}", round);
		self.known_votes.write().conclude(round);
	}
}

impl<B> Validator<B> for GossipValidator<B>
where
	B: Block,
{
	fn validate(
		&self,
		_context: &mut dyn ValidatorContext<B>,
		_sender: &PeerId,
		mut _data: &[u8],
	) -> ValidationResult<B::Hash> {
		// This passes message to worker
		return ValidationResult::ProcessAndKeep(self.topic);
		/*		if let Ok(msg) = TSSData::deserialize(&mut data) {
					let msg_hash = twox_64(data);

					if true {
						// TimeKeyvault::verify(&msg.id.clone().into(), &msg.signature, &msg.encode()) {
					} else {
						// TODO: report peer
						info!(target: TW_LOG, "Bad signature on message: {:?}, from: {:?}", msg, sender);
					}
				}

				ValidationResult::Discard
		*/
	}

	fn message_expired<'a>(&'a self) -> Box<dyn FnMut(B::Hash, &[u8]) -> bool + 'a> {
		let _known_votes = self.known_votes.read();
		Box::new(move |_topic, mut _data| {
			/*			let msg = match TSSData::deserialize(&mut data) {
							Ok(vote) => vote,
							Err(_) => return true,
						};
			*/
			false
		})
	}

	fn message_allowed<'a>(
		&'a self,
	) -> Box<dyn FnMut(&PeerId, MessageIntent, &B::Hash, &[u8]) -> bool + 'a> {
		let do_rebroadcast = {
			let now = Instant::now();
			let mut next_rebroadcast = self.next_rebroadcast.lock();
			if now >= *next_rebroadcast {
				*next_rebroadcast = now + REBROADCAST_AFTER;
				true
			} else {
				false
			}
		};

		let _known_votes = self.known_votes.read();
		Box::new(move |_who, intent, _topic, mut _data| {
			if let MessageIntent::PeriodicRebroadcast = intent {
				return do_rebroadcast;
			}

			/*			let msg = match TSSData::deserialize(&mut data) {
							Ok(vote) => vote,
							Err(_) => {
								error!(target: TW_LOG, "Failed to decode ping gossip");
								return false
							},
						};
			*/
			// FIXME: we can put entire TSS into validator and worker will only get fully signed
			// messages
			true
		})
	}
}
