use anyhow::{Context, Result};
use gmp::Backend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tc_subxt::MetadataVariant;
use time_primitives::NetworkId;

#[derive(Clone, Debug)]
pub struct Config {
	path: PathBuf,
	yaml: ConfigYaml,
}

impl Config {
	pub fn from_file(path: &Path) -> Result<Self> {
		let config = std::fs::read_to_string(path).context("failed to read config file")?;
		let yaml = serde_yaml::from_str(&config).context("failed to parse config file")?;
		Ok(Self {
			path: path.parent().unwrap().to_path_buf(),
			yaml,
		})
	}

	fn relative_path(&self, other: &Path) -> PathBuf {
		if other.is_absolute() {
			return other.to_owned();
		}
		self.path.join(other)
	}

	pub fn timechain_mnemonic(&self) -> Result<String> {
		let path = self.relative_path(&self.yaml.config.timechain_keyfile);
		std::fs::read_to_string(path).context("failed to read timechain keyfile")
	}

	pub fn target_mnemonic(&self) -> Result<String> {
		let path = self.relative_path(&self.yaml.config.target_keyfile);
		std::fs::read_to_string(path).context("failed to read target keyfile")
	}

	pub fn global(&self) -> &GlobalConfig {
		&self.yaml.config
	}

	pub fn chronicles(&self) -> &[String] {
		&self.yaml.chronicles
	}

	pub fn contracts(&self, network: NetworkId) -> Result<Contracts> {
		let network = self.network(network)?;
		Ok(if let Some(contracts) = self.yaml.contracts.get(&network.backend) {
			Contracts {
				proxy: std::fs::read(self.relative_path(&contracts.proxy))
					.context("failed to read proxy contract")?,
				gateway: std::fs::read(self.relative_path(&contracts.gateway))
					.context("failed to read gateway contract")?,
				tester: std::fs::read(self.relative_path(&contracts.tester))
					.context("failed to read tester contract")?,
			}
		} else {
			Contracts::default()
		})
	}

	pub fn networks(&self) -> &HashMap<NetworkId, NetworkConfig> {
		&self.yaml.networks
	}

	pub fn network(&self, network: NetworkId) -> Result<&NetworkConfig> {
		self.yaml.networks.get(&network).context("no network config")
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ConfigYaml {
	config: GlobalConfig,
	contracts: HashMap<Backend, ContractsConfig>,
	networks: HashMap<NetworkId, NetworkConfig>,
	chronicles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub chronicle_timechain_funds: u128,
	pub metadata_variant: MetadataVariant,
	timechain_keyfile: PathBuf,
	pub timechain_url: String,
	target_keyfile: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ContractsConfig {
	proxy: PathBuf,
	gateway: PathBuf,
	tester: PathBuf,
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
	pub symbol: String,
	pub route_gas_limit: u64,
	pub route_base_fee: u128,
}
