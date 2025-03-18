use crate::{Address32, Gateway};
use anyhow::{anyhow, Result};
use polkadot_sdk::{sp_core::ConstU32, sp_runtime::BoundedVec};
use scale_codec::{Decode, Encode};
use scale_info::prelude::string::String;
use scale_info::prelude::vec::Vec;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

pub const CHAIN_NAME_LEN: u32 = 50;
pub const CHAIN_NET_LEN: u32 = 50;
pub const MAX_CCTP_ADDRESSES: u32 = 50;
pub const MAX_CCTP_URL_LEN: u32 = 200;

pub type NetworkId = u16;
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ChainName(pub BoundedVec<u8, ConstU32<CHAIN_NAME_LEN>>);
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct ChainNetwork(pub BoundedVec<u8, ConstU32<CHAIN_NET_LEN>>);
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CctpContracts(pub BoundedVec<Address32, ConstU32<MAX_CCTP_ADDRESSES>>);
#[derive(Encode, Decode, TypeInfo, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CctpUrl(pub BoundedVec<u8, ConstU32<MAX_CCTP_URL_LEN>>);

impl CctpContracts {
	pub fn push_unique(&mut self, new_contract: Address32) -> Result<(), anyhow::Error> {
		if !self.0.contains(&new_contract) {
			self.0
				.try_push(new_contract)
				.map_err(|e| anyhow::anyhow!("failed to add new contract: {:?}", e))?;
		}
		Ok(())
	}
}

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
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub cctp_contracts: Option<CctpContracts>,
	pub cctp_url: Option<CctpUrl>,
}

#[cfg(feature = "std")]
impl TryFrom<Vec<String>> for CctpContracts {
	type Error = anyhow::Error;
	fn try_from(contracts: Vec<String>) -> Result<Self> {
		let addresses: Result<Vec<Address32>> = contracts
			.into_iter()
			.map(|addr_str| {
				let clean = addr_str.trim().trim_start_matches("0x");
				let bytes = hex::decode(clean).map_err(|_| {
					anyhow::anyhow!("Unable to decode hex for address: {}", addr_str)
				})?;
				let address: Address32 = bytes.try_into().map_err(|_| {
					anyhow!("Unable to convert bytes to address format for: {}", addr_str)
				})?;
				Ok(address)
			})
			.collect();

		let addresses = addresses?;
		let bounded_addresses = BoundedVec::try_from(addresses)
			.map_err(|_| anyhow!("Exceeded maximum of {} CCTP addresses", MAX_CCTP_ADDRESSES))?;
		Ok(CctpContracts(bounded_addresses))
	}
}

#[cfg(feature = "std")]
impl TryFrom<&str> for CctpUrl {
	type Error = anyhow::Error;

	fn try_from(s: &str) -> Result<Self> {
		let url_bytes = s.as_bytes().to_vec();
		let bounded_url = BoundedVec::try_from(url_bytes).map_err(|_| {
			anyhow!("URL length {} exceeds maximum of {} bytes", s.len(), MAX_CCTP_URL_LEN)
		})?;
		Ok(CctpUrl(bounded_url))
	}
}
