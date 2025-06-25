use anyhow::{Context, Result};
use csv::{Reader, Writer};
use gmp::Backend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use time_primitives::{Currency, NetworkId};

#[derive(Clone, Debug)]
pub struct Config {
	path: PathBuf,
	yaml: ConfigYaml,
	prices: HashMap<NetworkId, (String, f64)>,
	testers: HashMap<NetworkId, (String, u64)>,
}

#[derive(Clone, Deserialize)]
struct NetworkPrice {
	pub network_id: NetworkId,
	pub symbol: String,
	pub usd_price: f64,
}

#[derive(Clone, Deserialize)]
struct Tester {
	pub network_id: NetworkId,
	pub address: String,
	pub block: u64,
}

pub fn write_prices(path: &Path, prices: &HashMap<NetworkId, (String, f64)>) -> Result<()> {
	let file =
		File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
	let mut wtr = Writer::from_writer(file);
	wtr.write_record(["network_id", "symbol", "usd_price"])?;
	for (network, (symbol, usd_price)) in prices {
		wtr.write_record(&[network.to_string(), symbol.to_string(), usd_price.to_string()])?;
	}
	wtr.flush()?;
	Ok(())
}

pub fn write_testers(path: &Path, testers: &HashMap<NetworkId, (String, u64)>) -> Result<()> {
	let file =
		File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
	let mut wtr = Writer::from_writer(file);
	wtr.write_record(["network_id", "address", "block"])?;
	for (network, (address, block)) in testers {
		wtr.write_record(&[network.to_string(), address.to_string(), block.to_string()])?;
	}
	wtr.flush()?;
	Ok(())
}

impl Config {
	pub fn from_env(path: PathBuf, config: &str) -> Result<Self> {
		let config_path = path.join(config);
		let config = std::fs::read_to_string(&config_path)
			.with_context(|| format!("failed to read config file {}", config_path.display()))?;
		let yaml = serde_yaml::from_str(&config)
			.with_context(|| format!("failed to parse config file {}", config_path.display()))?;
		let mut me = Self {
			path,
			yaml,
			prices: Default::default(),
			testers: Default::default(),
		};
		me.load_prices()?;
		me.load_testers()?;
		Ok(me)
	}

	pub fn prefix(&self) -> Option<String> {
		let path = std::fs::canonicalize(&self.path).ok()?;
		let prefix = path.file_name()?.to_str()?.strip_prefix('.')?;
		if prefix.starts_with("tmp") {
			Some(format!("{prefix}-"))
		} else {
			None
		}
	}

	fn relative_path(&self, other: &Path) -> PathBuf {
		if other.is_absolute() {
			return other.to_owned();
		}
		self.path.join(other)
	}

	pub fn token_price_usd(&self, network: NetworkId) -> Result<f64> {
		self.prices
			.get(&network)
			.map(|(_, price)| *price)
			.ok_or_else(|| anyhow::anyhow!("No token price data for network {}", network))
	}

	pub fn balance_to_usd(&self, network: NetworkId, balance: u128) -> Result<f64> {
		let token_price = self.token_price_usd(network)?;
		let decimals = self.network(network)?.currency_decimals;
		let factor = 10.0f64.powi(decimals as i32);
		Ok(balance as f64 / factor * token_price)
	}

	/// Destination network gas price expressed in source network token
	pub fn gas_price(&self, src_network: NetworkId, dest_network: NetworkId) -> Result<f64> {
		let usd_src = self.token_price_usd(src_network)?;
		let usd_dest = self.token_price_usd(dest_network)?;
		let src = self.network(src_network)?;
		let dest = self.network(dest_network)?;
		Ok(gas_price(
			usd_src,
			src.currency_decimals,
			usd_dest,
			dest.currency_decimals,
			dest.max_gas_price,
		))
	}

	/// Fee paid for sending a message.
	pub fn msg_fee(
		&self,
		src_network: NetworkId,
		dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
	) -> Result<u128> {
		let dest = self.network(dest_network)?;
		let gas = dest.gas(msg_size, gas_limit);
		let gas_price = self.gas_price(src_network, dest_network)?;
		Ok((gas as f64 * gas_price + dest.route_msg_fee as f64) as u128)
	}

	pub fn load_prices(&mut self) -> Result<()> {
		let price_path = self.relative_path(&self.yaml.config.prices_path);
		if !price_path.exists() {
			return Ok(());
		}
		let mut rdr = Reader::from_path(&price_path)
			.with_context(|| format!("failed to open {}", price_path.display()))?;

		for result in rdr.deserialize() {
			let record: NetworkPrice = result?;
			self.prices.insert(record.network_id, (record.symbol, record.usd_price));
		}
		Ok(())
	}

	pub fn save_prices(&mut self, prices: HashMap<NetworkId, (String, f64)>) -> Result<()> {
		let price_path = self.relative_path(&self.yaml.config.prices_path);
		write_prices(&price_path, &prices)?;
		self.prices = prices;
		Ok(())
	}

	pub fn tester(&self, network: NetworkId) -> Option<&(String, u64)> {
		self.testers.get(&network)
	}

	pub fn load_testers(&mut self) -> Result<()> {
		let testers_path = self.relative_path(&self.yaml.config.testers_path);
		if !testers_path.exists() {
			return Ok(());
		}
		let mut rdr = Reader::from_path(&testers_path)
			.with_context(|| format!("failed to open {}", testers_path.display()))?;

		for result in rdr.deserialize() {
			let record: Tester = result?;
			self.testers.insert(record.network_id, (record.address, record.block));
		}
		Ok(())
	}

	pub fn save_testers(&mut self, testers: HashMap<NetworkId, (String, u64)>) -> Result<()> {
		let testers_path = self.relative_path(&self.yaml.config.testers_path);
		write_testers(&testers_path, &testers)?;
		self.testers = testers;
		Ok(())
	}

	pub fn global(&self) -> &GlobalConfig {
		&self.yaml.config
	}

	pub fn chronicles(&self) -> &[String] {
		&self.yaml.chronicles
	}

	pub fn backend(&self, network: NetworkId) -> Result<BackendData> {
		let network = self.network(network)?;
		Ok(if let Some(backend) = self.yaml.backends.get(&network.backend) {
			BackendData {
				proxy: {
					let path = self.relative_path(&backend.proxy);
					std::fs::read(&path).with_context(|| {
						format!("failed to read proxy contract from {}", path.display())
					})?
				},
				gateway: {
					let path = self.relative_path(&backend.gateway);
					std::fs::read(&path).with_context(|| {
						format!("failed to read gateway contract from {}", path.display())
					})?
				},
				tester: {
					let path = self.relative_path(&backend.tester);
					std::fs::read(&path).with_context(|| {
						format!("failed to read tester contract from {}", path.display())
					})?
				},
			}
		} else {
			BackendData::default()
		})
	}

	pub fn networks(&self) -> &HashMap<NetworkId, NetworkConfig> {
		&self.yaml.networks
	}

	pub fn network(&self, network: NetworkId) -> Result<&NetworkConfig> {
		self.yaml.networks.get(&network).context("no network config")
	}
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigYaml {
	pub config: GlobalConfig,
	pub backends: HashMap<Backend, BackendConfig>,
	pub networks: HashMap<NetworkId, NetworkConfig>,
	pub chronicles: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalConfig {
	pub prices_path: PathBuf,
	pub testers_path: PathBuf,
	pub chronicle_funds: String,
	pub timechain_url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendConfig {
	pub proxy: PathBuf,
	pub gateway: PathBuf,
	pub tester: PathBuf,
}

#[derive(Default)]
pub struct BackendData {
	pub proxy: Vec<u8>,
	pub gateway: Vec<u8>,
	pub tester: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
	pub backend: Backend,
	pub name: String,
	pub url: String,
	pub coin_id: String,
	pub currency_decimals: u8,
	pub currency_symbol: String,
	pub admin_funds: Option<String>,
	pub gateway_funds: String,
	pub chronicle_funds: String,
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub shard_task_limit: u32,
	pub batch_size: u32,
	pub batch_offset: u32,
	pub batch_gas_limit: u64,
	pub route_max_gas_limit: u64,
	pub route_msg_fee: u64,
	pub batch_exec_gas: u64,
	pub reg_op_exec_gas: u64,
	pub unreg_op_exec_gas: u64,
	pub msg_op_exec_gas: u64,
	pub msg_session_gas: u64,
	pub msg_byte_gas: u64,
	pub max_gas_price: u128,
}

fn gas_price(
	usd_src: f64,
	src_decimals: u8,
	usd_dest: f64,
	dest_decimals: u8,
	dest_max_gas_price: u128,
) -> f64 {
	usd_dest / usd_src
		* dest_max_gas_price as f64
		* f64::powi(10., src_decimals as i32 - dest_decimals as i32)
}

impl NetworkConfig {
	pub fn num_sessions(&self) -> u16 {
		self.shard_size - self.shard_threshold + 1
	}

	pub fn msg_gas(&self) -> u64 {
		self.num_sessions() as u64 * self.msg_session_gas
			+ self.batch_exec_gas
			+ self.msg_op_exec_gas
			- self.msg_session_gas
	}

	pub fn msg_byte_gas(&self) -> u64 {
		self.num_sessions() as u64 * self.msg_byte_gas
	}

	pub fn gas(&self, msg_size: u16, gas_limit: u64) -> u64 {
		self.msg_byte_gas() * msg_size as u64 + self.msg_gas() + gas_limit
	}

	pub fn currency(&self) -> Currency {
		Currency::new(self.currency_decimals, self.currency_symbol.clone())
	}
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
				prices.extend(config.prices.keys().copied());
			}
			assert_eq!(prices, networks, "{}", env_dir.path().display());
		}
		Ok(())
	}
}
