use anyhow::{Context, Result};
use gmp::Backend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use time_primitives::NetworkId;

#[derive(Clone, Debug)]
pub struct Config {
	path: PathBuf,
	yaml: ConfigYaml,
}

impl Config {
	pub fn from_env(path: PathBuf, config: &str) -> Result<Self> {
		let config_path = path.join(config);
		let config = std::fs::read_to_string(&config_path)
			.with_context(|| format!("failed to read config file {}", config_path.display()))?;
		let yaml = serde_yaml::from_str(&config)
			.with_context(|| format!("failed to parse config file {}", config_path.display()))?;
		Ok(Self { path, yaml })
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
				proxy: {
					let path = self.relative_path(&contracts.proxy);
					std::fs::read(&path).with_context(|| {
						format!("failed to read proxy contract from {}", path.display())
					})?
				},
				gateway: {
					let path = self.relative_path(&contracts.gateway);
					std::fs::read(&path).with_context(|| {
						format!("failed to read gateway contract from {}", path.display())
					})?
				},
				tester: {
					let path = self.relative_path(&contracts.tester);
					std::fs::read(&path).with_context(|| {
						format!("failed to read tester contract from {}", path.display())
					})?
				},
				additional_params: {
					let path = self.relative_path(&contracts.additional_params);
					std::fs::read(&path).with_context(|| {
						format!("failed to read additional params from {}", path.display())
					})?
				},
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
#[serde(deny_unknown_fields)]
struct ConfigYaml {
	config: GlobalConfig,
	contracts: HashMap<Backend, ContractsConfig>,
	networks: HashMap<NetworkId, NetworkConfig>,
	chronicles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
	prices_path: PathBuf,
	pub chronicle_timechain_funds: String,
	pub timechain_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractsConfig {
	additional_params: PathBuf,
	proxy: PathBuf,
	gateway: PathBuf,
	tester: PathBuf,
}

#[derive(Default)]
pub struct Contracts {
	pub additional_params: Vec<u8>,
	pub proxy: Vec<u8>,
	pub gateway: Vec<u8>,
	pub tester: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub faucet: Option<String>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashSet;

	#[test]
	fn make_sure_envs_parse() -> Result<()> {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../config/envs");
		let envs = std::fs::read_dir(&root)?;
		for env in envs {
			let env_dir = env?;
			if !env_dir.file_type()?.is_dir() {
				continue;
			}
			let mut networks = HashSet::new();
			let mut prices = HashSet::new();
			println!("env {}", env_dir.file_name().into_string().unwrap());
			for config in std::fs::read_dir(env_dir.path())? {
				let config = config?;
				if !config.file_type()?.is_file() {
					continue;
				}
				let config = config.file_name().into_string().unwrap();
				if !config.ends_with(".yaml") {
					continue;
				}
				println!("  config {}", config);
				let config = Config::from_env(env_dir.path(), &config).unwrap();
				networks.extend(config.networks().keys().copied());
				let prices_path = config.prices();
				let prices_csv = crate::gas_price::read_csv_token_prices(&prices_path).unwrap();
				prices.extend(prices_csv.keys().copied());
			}
			assert_eq!(prices, networks, "{}", env_dir.path().display());
		}
		Ok(())
	}
}
