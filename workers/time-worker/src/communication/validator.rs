use sc_network::PeerId;
use sc_network_gossip::{ValidationResult, Validator, ValidatorContext};
use sp_runtime::traits::{Block, Hash, Header};

/// Gossip engine messages topic
pub(crate) fn topic<B: Block>() -> B::Hash
where
	B: Block,
{
	<<B::Header as Header>::Hashing as Hash>::hash(b"time-tss")
}

pub(crate) struct GossipValidator<B>
where
	B: Block,
{
	topic: B::Hash,
}

impl<B> GossipValidator<B>
where
	B: Block,
{
	pub fn new() -> GossipValidator<B> {
		GossipValidator { topic: topic::<B>() }
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
		_data: &[u8],
	) -> ValidationResult<B::Hash> {
		ValidationResult::ProcessAndDiscard(self.topic)
	}
}
