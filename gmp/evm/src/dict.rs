use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

type ChainDict = HashMap<u64, Chain>;

#[derive(Deserialize)]
pub struct Chain {
	_chain_id: u64,
	_name: String,
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

pub(crate) fn load() -> Result<ChainDict> {
    // taken from https://chainid.network/chains.json
	let json = std::fs::read_to_string("auxiliary/chains.json")?;
	Ok(serde_json::from_str(&json)?)
}
