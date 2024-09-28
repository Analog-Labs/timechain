use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tc_subxt::MetadataVariant;
use time_primitives::NetworkId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub config: GlobalConfig,
	pub contracts: HashMap<Backend, ContractsConfig>,
	pub networks: HashMap<NetworkId, NetworkConfig>,
	pub chronicles: Vec<String>,
}

impl Config {
	pub fn from_file(path: &Path) -> Result<Self> {
		let config = std::fs::read_to_string(path)?;
		Ok(serde_yaml::from_str(&config)?)
	}

	pub fn contracts(&self, network: NetworkId) -> Result<Contracts> {
		let network = self.network(network)?;
		Ok(if let Some(contracts) = self.contracts.get(&network.backend) {
			contracts.load()?
		} else {
			Contracts::default()
		})
	}

	pub fn network(&self, network: NetworkId) -> Result<&NetworkConfig> {
		self.networks.get(&network).context("no network config")
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub chronicle_timechain_funds: u128,
	pub metadata_variant: MetadataVariant,
	pub timechain_keyfile: PathBuf,
	pub timechain_url: String,
	pub target_keyfile: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
	Grpc,
	Evm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractsConfig {
	pub proxy: PathBuf,
	pub gateway: PathBuf,
	pub tester: PathBuf,
}

impl ContractsConfig {
	fn load(&self) -> Result<Contracts> {
		Ok(Contracts {
			proxy: std::fs::read(&self.proxy)?,
			gateway: std::fs::read(&self.gateway)?,
			tester: std::fs::read(&self.tester)?,
		})
	}
}

#[derive(Default)]
pub struct Contracts {
	pub proxy: Vec<u8>,
	pub gateway: Vec<u8>,
	pub tester: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
	pub backend: Backend,
	pub blockchain: String,
	pub network: String,
	pub url: String,
	pub gateway_funds: u128,
	pub chronicle_target_funds: u128,
	pub batch_size: u32,
	pub batch_offset: u32,
	pub batch_gas_limit: u128,
	pub shard_task_limit: u32,
	pub route_gas_limit: u64,
	pub route_base_fee: u128,
	pub route_gas_price: Ratio,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ratio {
	pub num: u128,
	pub den: u128,
}
