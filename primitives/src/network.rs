use crate::Gateway;
use scale_codec::{Decode, Encode};
use scale_info::prelude::string::String;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

pub type NetworkId = u16;
pub type ChainName = String;
pub type ChainNetwork = String;

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct Network {
	pub id: NetworkId,
	pub chain_name: ChainName,
	pub chain_network: ChainNetwork,
	pub gateway: Gateway,
	pub gateway_block: u64,
	pub config: NetworkConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct NetworkConfig {
	pub batch_size: u32,
	pub batch_offset: u32,
	pub batch_gas_limit: u128,
	pub shard_task_limit: u32,
}
