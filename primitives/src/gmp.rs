use crate::{NetworkId, ShardId, TaskId, TssPublicKey};
use scale_codec::{Decode, Encode};
use scale_decode::DecodeAsType;
use scale_info::{prelude::vec::Vec, TypeInfo};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type Gateway = [u8; 20];

#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct GmpParams {
	pub network: NetworkId,
	pub gateway: Gateway,
	pub signer: TssPublicKey,
}

impl GmpParams {
	pub fn domain_separator(&self) -> [u8; 32] {
		todo!()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, Decode, DecodeAsType, Encode, TypeInfo, PartialEq)]
pub struct GmpMessage {
	pub src_network: NetworkId,
	pub src: [u8; 32],
	pub dest_network: NetworkId,
	pub dest: [u8; 32],
	pub nonce: u64,
	pub gas_limit: u128,
	pub bytes: Vec<u8>,
}

impl GmpMessage {
	pub fn message_id(&self) -> [u8; 32] {
		todo!()
	}
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub enum GatewayOp {
	RegisterShard(
		ShardId,
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	UnregisterShard(
		ShardId,
		#[cfg_attr(feature = "std", serde(with = "crate::shard::serde_tss_public_key"))]
		TssPublicKey,
	),
	SendMessage(GmpMessage),
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Decode, Encode, TypeInfo, PartialEq)]
pub struct GatewayMessage {
	pub ops: Vec<GatewayOp>,
	pub task_id: TaskId,
}

impl GatewayMessage {
	pub fn new(task_id: TaskId, ops: Vec<GatewayOp>) -> Self {
		Self { task_id, ops }
	}

	pub fn hash(&self, _params: &GmpParams) -> [u8; 32] {
		todo!()
	}
}
