use crate::{GatewayOp, GmpEvent, NetworkId, ShardId, TssSignature};
use scale_codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type TaskId = u64;
pub type TaskIndex = u64;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Function {
	ReadGatewayEvents { batch_size: core::num::NonZeroU64 },
	SignPayload { payload: Vec<u8> },
	SubmitGatewayMessage { ops: Vec<GatewayOp> },
}

#[cfg(feature = "std")]
impl std::fmt::Display for Function {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Function::ReadGatewayEvents { batch_size } => {
				write!(f, "ReadGatewayEvents({batch_size})")
			},
			Function::SubmitGatewayMessage { ops: _ } => write!(f, "SubmitGatewayMessage"),
			Function::SignPayload { payload } => {
				let len = payload.len();
				write!(f, "SignPayload({len})")
			},
		}
	}
}

impl Function {
	pub fn get_input_length(&self) -> u32 {
		self.encoded_size() as _
	}
}

#[derive(Debug, Clone, Decode, DecodeAsType, Encode, TypeInfo, PartialEq)]
pub struct TaskResult {
	pub shard_id: ShardId,
	pub payload: Vec<GmpEvent>,
	pub signature: TssSignature,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct TaskDescriptor {
	pub network: NetworkId,
	pub function: Function,
	pub start: u64,
}

impl TaskDescriptor {
	pub fn new(network: NetworkId, function: Function) -> Self {
		Self { network, function, start: 0 }
	}
}
