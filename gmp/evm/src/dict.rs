use anyhow::Result;
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};

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

pub(crate) fn load(path: PathBuf) -> Result<ChainDict> {
	let json = std::fs::read_to_string(path)?;
	let chains: Vec<Chain> = serde_json::from_str(&json)?;

	Ok(chains.into_iter().map(|c| (c.chain_id, c)).collect::<_>())
}
