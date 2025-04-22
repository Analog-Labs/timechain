use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use time_primitives::NetworkId;

type ChainDict = HashMap<u64, Chain>;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Chain {
	chain_id: u64,
	name: String,
	pub currency: Currency,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Currency {
	pub name: String,
	pub symbol: String,
	pub decimals: u8,
}

impl Default for Currency {
	fn default() -> Self {
		Currency {
			name: "Ether".to_string(),
			symbol: "ETH".to_string(),
			decimals: 18,
		}
	}
}

pub(crate) fn load(data: &[u8]) -> Result<ChainDict> {
	let chains: Vec<Chain> = serde_json::from_slice(data)?;
	Ok(chains.into_iter().map(|c| (c.chain_id, c)).collect::<_>())
}

pub(crate) fn network_id_to_domain_id(network_id: NetworkId) -> Result<u32> {
	match network_id {
		10 => Ok(0),
		13 => Ok(3),
		_ => anyhow::bail!("Unsupported chain id"),
	}
}
