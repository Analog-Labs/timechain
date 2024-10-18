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
		let config = std::fs::read_to_string(path)
			.with_context(|| format!("failed to read config file {}", path.display()))?;
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

	pub fn prices(&self) -> PathBuf {
		self.relative_path(&self.yaml.config.prices_path)
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
	prices_path: PathBuf,
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub chronicle_timechain_funds: String,
	pub metadata_variant: MetadataVariant,
	pub timechain_url: String,
	pub factory_deployer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ContractsConfig {
	factory: PathBuf,
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
	pub gateway_funds: String,
	pub chronicle_target_funds: String,
	pub batch_size: u32,
	pub batch_offset: u32,
	pub batch_gas_limit: u128,
	pub gmp_margin: f64,
	pub shard_task_limit: u32,
	pub route_gas_limit: u64,
	pub route_base_fee: u128,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn make_sure_envs_parse() {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../config/envs");
		Config::from_file(&root.join("local/config.yaml")).unwrap();
		Config::from_file(&root.join("development/config.yaml")).unwrap();
	}
}
