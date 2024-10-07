use crate::{BatchId, GmpEvent, TssSignature, ANLOG};
use core::ops::Range;
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedVec};
use scale_codec::{Decode, Encode};
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type TaskId = u64;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum Task {
	ReadGatewayEvents { blocks: Range<u64> },
	SubmitGatewayMessage { batch_id: BatchId },
}

#[cfg(feature = "std")]
impl std::fmt::Display for Task {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::ReadGatewayEvents { blocks } => {
				let start = blocks.start;
				let end = blocks.end;
				write!(f, "ReadGatewayEvents({start}..{end})")
			},
			Self::SubmitGatewayMessage { batch_id } => {
				write!(f, "SubmitGatewayMessage({batch_id})")
			},
		}
	}
}

impl Task {
	pub fn get_input_length(&self) -> u32 {
		self.encoded_size() as _
	}

	pub fn reward(&self) -> u128 {
		2 * ANLOG
	}

	pub fn needs_registration(&self) -> bool {
		!matches!(self, Self::ReadGatewayEvents { .. })
	}

	pub fn needs_signer(&self) -> bool {
		matches!(self, Self::SubmitGatewayMessage { .. })
	}

	pub fn start_block(&self) -> u64 {
		if let Self::ReadGatewayEvents { blocks } = self {
			blocks.end
		} else {
			0
		}
	}
}

pub fn encode_gmp_events(task_id: TaskId, events: &[GmpEvent]) -> Vec<u8> {
	(task_id, events).encode()
}

const MAX_GMP_EVENTS: u32 = 1_000;
const MAX_ERROR_LEN: u32 = 500;
/// Bounded vec alias for GMP events submitted in results
pub type GmpEvents = BoundedVec<GmpEvent, ConstU32<MAX_GMP_EVENTS>>;
/// Bounded vec alias for SubmitGatewayMessage error
pub type ErrorMsg = BoundedVec<u8, ConstU32<MAX_ERROR_LEN>>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum TaskResult {
	ReadGatewayEvents {
		events: GmpEvents,
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_signature"))]
		signature: TssSignature,
	},
	SubmitGatewayMessage {
		error: ErrorMsg,
	},
}
