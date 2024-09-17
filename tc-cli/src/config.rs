use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use time_primitives::NetworkId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
	pub backends: HashMap<Backend, BackendConfig>,
	pub networks: HashMap<NetworkId, NetworkConfig>,
}

impl Config {
	pub fn from_file(path: &Path) -> Result<Self> {
		let config = std::fs::read_to_string(path)?;
		Ok(serde_yaml::from_str(&config)?)
	}

	pub fn backend(&self, network: NetworkId) -> Result<&BackendConfig> {
		let network = self.networks.get(&network).context("no network config")?;
		self.backends.get(&network.backend).context("no backend config")
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Backend {
	Rust,
	Evm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendConfig {
	pub proxy: PathBuf,
	pub gateway: PathBuf,
	pub tester: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
	pub backend: Backend,
	pub url: String,
}
