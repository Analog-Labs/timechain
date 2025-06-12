use crate::{Address32, BatchGasParams};
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedVec};
use scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

pub const CHAIN_NAME_LEN: u32 = 50;

pub type NetworkId = u16;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	PartialEq,
	Eq,
	Clone,
	Debug,
	Serialize,
	Deserialize,
)]
pub struct ChainName(pub BoundedVec<u8, ConstU32<CHAIN_NAME_LEN>>);

#[derive(
	Clone,
	Debug,
	Eq,
	PartialEq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub struct Network {
	pub id: NetworkId,
	pub chain_name: ChainName,
	pub gateway: Address32,
	pub gateway_block: u64,
	pub config: NetworkConfig,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Serialize,
	Deserialize,
)]
pub struct NetworkConfig {
	pub batch_size: u32,
	pub batch_offset: u32,
	pub shard_task_limit: u32,
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub batch_gas_params: BatchGasParams,
	pub max_gas_price: u128,
}
