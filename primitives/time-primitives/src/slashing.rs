use crate::TimeId;

/// Identifier of current protocol and stage
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Protocol {
	KgStageOne,
	KgStageTwo,
	Signing,
}

/// Structure to identify offender of any protocol in given shard
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct TimeoutData {
	pub protocol: Protocol,
	pub offender: TimeId,
	pub shard_id: u64,
}

impl TimeoutData {
	// Constructor for convenience
	pub fn new(protocol: Protocol, offender: TimeId, shard_id: u64) -> Self {
		TimeoutData { protocol, offender, shard_id }
	}
}
