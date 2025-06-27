use anyhow::{Context, Result};
use csv::{Reader, Writer};
use gmp::Backend;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time_primitives::{Currency, NetworkId};

#[derive(Clone, Deserialize)]
pub struct Price {
	pub network_id: NetworkId,
	pub usd_price: f64,
}

impl Price {
	fn read(path: &Path) -> Result<HashMap<NetworkId, f64>> {
		if !path.exists() {
			return Ok(Default::default());
		}
		let mut rdr = Reader::from_path(path)
			.with_context(|| format!("failed to open {}", path.display()))?;

		let mut csv = HashMap::new();
		for result in rdr.deserialize() {
			let result: Self = result?;
			csv.insert(result.network_id, result.usd_price);
		}
		Ok(csv)
	}

	pub fn write(path: &Path, prices: &HashMap<NetworkId, f64>) -> Result<()> {
		let file =
			File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
		let mut wtr = Writer::from_writer(file);
		wtr.write_record(["network_id", "usd_price"])?;
		for (network, usd_price) in prices {
			wtr.write_record(&[network.to_string(), usd_price.to_string()])?;
		}
		wtr.flush()?;
		Ok(())
	}
}

#[derive(Clone, Deserialize)]
pub struct Tester {
	pub network_id: NetworkId,
	pub address: String,
}

impl Tester {
	fn read(path: &Path) -> Result<HashMap<NetworkId, String>> {
		if !path.exists() {
			return Ok(Default::default());
		}
		let mut rdr = Reader::from_path(path)
			.with_context(|| format!("failed to open {}", path.display()))?;

		let mut csv = HashMap::new();
		for result in rdr.deserialize() {
			let result: Self = result?;
			csv.insert(result.network_id, result.address);
		}
		Ok(csv)
	}

	pub fn write(path: &Path, testers: &HashMap<NetworkId, String>) -> Result<()> {
		let file =
			File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
		let mut wtr = Writer::from_writer(file);
		wtr.write_record(["network_id", "address"])?;
		for (network, address) in testers {
			wtr.write_record(&[network.to_string(), address.clone()])?;
		}
		wtr.flush()?;
		Ok(())
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfigYaml {
	pub config: GlobalConfigYaml,
	pub backends: HashMap<Backend, BackendConfigYaml>,
	pub networks: HashMap<NetworkId, NetworkConfigYaml>,
	pub chronicles: Vec<String>,
}

impl ConfigYaml {
	fn read(path: &Path) -> Result<Self> {
		let config = std::fs::read_to_string(path)
			.with_context(|| format!("failed to read config file {}", path.display()))?;
		let yaml = serde_yaml::from_str(&config)
			.with_context(|| format!("failed to parse config file {}", path.display()))?;
		Ok(yaml)
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalConfigYaml {
	pub prices_path: PathBuf,
	pub testers_path: PathBuf,
	pub chronicle_funds: String,
	pub timechain_url: String,
	#[serde(flatten)]
	pub inherited: InheritableGlobalYaml,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InheritableGlobalYaml {
	pub shard_size: Option<u16>,
	pub shard_threshold: Option<u16>,
	pub shard_task_limit: Option<u32>,
	pub msg_fee: Option<f64>,
}

impl InheritableGlobalYaml {
	fn inherit(&mut self, inherited: &InheritableGlobalYaml) {
		if self.shard_size.is_none() {
			self.shard_size = inherited.shard_size;
		}
		if self.shard_threshold.is_none() {
			self.shard_threshold = inherited.shard_threshold;
		}
		if self.shard_task_limit.is_none() {
			self.shard_task_limit = inherited.shard_task_limit;
		}
		if self.msg_fee.is_none() {
			self.msg_fee = inherited.msg_fee;
		}
	}
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BackendConfigYaml {
	pub proxy: Option<PathBuf>,
	pub gateway: Option<PathBuf>,
	pub tester: Option<PathBuf>,
	pub batch_exec_gas: u64,
	pub reg_op_exec_gas: u64,
	pub unreg_op_exec_gas: u64,
	pub msg_op_exec_gas: u64,
	pub msg_session_gas: u64,
	pub msg_byte_gas: u64,
	#[serde(flatten)]
	pub inherited: InheritableBackendYaml,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InheritableBackendYaml {
	pub batch_gas_limit: Option<f64>,
	pub msg_gas_limit: Option<f64>,
	#[serde(flatten)]
	pub inherited: InheritableGlobalYaml,
}

impl InheritableBackendYaml {
	fn inherit(&mut self, inherited: &InheritableBackendYaml) {
		if self.batch_gas_limit.is_none() {
			self.batch_gas_limit = inherited.batch_gas_limit;
		}
		if self.msg_gas_limit.is_none() {
			self.msg_gas_limit = inherited.msg_gas_limit;
		}
		self.inherited.inherit(&inherited.inherited);
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfigYaml {
	pub backend: Backend,
	pub name: String,
	pub url: String,
	pub coin_id: String,
	pub currency_decimals: u8,
	pub currency_symbol: String,
	pub admin_funds: Option<String>,
	pub gateway_funds: String,
	pub chronicle_funds: String,
	pub batch_size: u32,
	pub batch_offset: u32,
	pub max_gas_price: u128,
	pub block_gas_limit: u64,
	#[serde(flatten)]
	pub inherited: InheritableBackendYaml,
}

#[derive(Clone, Debug)]
pub struct Config {
	prefix: Option<String>,
	prices_path: PathBuf,
	testers_path: PathBuf,
	timechain_url: String,
	chronicle_funds: String,
	networks: HashMap<NetworkId, NetworkConfig>,
	chronicles: Vec<String>,
}

impl Config {
	pub fn from_env(path: &Path, config: &str) -> Result<Self> {
		let path = std::fs::canonicalize(path)?;
		let prefix = (|| {
			let prefix = path.file_name()?.to_str()?.strip_prefix('.')?;
			if prefix.starts_with("tmp") {
				Some(format!("{prefix}-"))
			} else {
				None
			}
		})();
		let config = ConfigYaml::read(&path.join(config))?;
		let relative_path = |other: &Path| -> PathBuf {
			if other.is_absolute() {
				return other.to_owned();
			}
			path.join(other)
		};
		let prices_path = relative_path(&config.config.prices_path);
		let testers_path = relative_path(&config.config.testers_path);
		let prices = Price::read(&prices_path)?;
		let testers = Tester::read(&testers_path)?;
		let mut backends = HashMap::new();
		#[derive(Clone, Default)]
		struct BackendConfig {
			proxy: Arc<[u8]>,
			gateway: Arc<[u8]>,
			tester: Arc<[u8]>,
			config: BackendConfigYaml,
		}
		let contract = |path: Option<&Path>| -> Result<Arc<[u8]>> {
			Ok(if let Some(path) = path {
				std::fs::read(relative_path(path))
					.with_context(|| format!("failed to read contract from {}", path.display()))?
					.into()
			} else {
				Default::default()
			})
		};
		for (network, mut backend) in config.backends {
			backend.inherited.inherited.inherit(&config.config.inherited);
			backends.insert(
				network,
				BackendConfig {
					proxy: contract(backend.proxy.as_deref())?,
					gateway: contract(backend.gateway.as_deref())?,
					tester: contract(backend.tester.as_deref())?,
					config: backend,
				},
			);
		}
		let mut networks = HashMap::new();
		for (network, mut yaml) in config.networks {
			let backend = backends.get(&yaml.backend).cloned().unwrap_or_default();
			yaml.inherited.inherit(&backend.config.inherited);
			networks.insert(
				network,
				NetworkConfig {
					backend: yaml.backend,
					proxy: backend.proxy.clone(),
					gateway: backend.gateway.clone(),
					tester: backend.tester.clone(),
					name: yaml.name,
					url: yaml.url,
					coin_id: yaml.coin_id,
					currency_decimals: yaml.currency_decimals,
					currency_symbol: yaml.currency_symbol,
					admin_funds: yaml.admin_funds,
					gateway_funds: yaml.gateway_funds,
					chronicle_funds: yaml.chronicle_funds,
					batch_size: yaml.batch_size,
					batch_offset: yaml.batch_offset,
					max_gas_price: yaml.max_gas_price,
					block_gas_limit: yaml.block_gas_limit,
					shard_size: yaml
						.inherited
						.inherited
						.shard_size
						.context("no shard_size specified")?,
					shard_threshold: yaml
						.inherited
						.inherited
						.shard_threshold
						.context("no shard_threshold specified")?,
					shard_task_limit: yaml
						.inherited
						.inherited
						.shard_task_limit
						.context("no shard_task_limit specified")?,
					batch_gas_limit: yaml
						.inherited
						.batch_gas_limit
						.context("no batch_gas_limit specified")?,
					msg_gas_limit: yaml
						.inherited
						.msg_gas_limit
						.context("no msg_gas_limit specified")?,
					msg_fee: yaml.inherited.inherited.msg_fee.context("no msg_fee specified")?,
					batch_exec_gas: backend.config.batch_exec_gas,
					reg_op_exec_gas: backend.config.reg_op_exec_gas,
					msg_op_exec_gas: backend.config.msg_op_exec_gas,
					unreg_op_exec_gas: backend.config.unreg_op_exec_gas,
					msg_session_gas: backend.config.msg_session_gas,
					msg_byte_gas: backend.config.msg_byte_gas,
					token_price_usd: prices.get(&network).copied(),
					tester_address: testers.get(&network).cloned(),
				},
			);
		}
		Ok(Self {
			prefix,
			prices_path,
			testers_path,
			timechain_url: config.config.timechain_url,
			chronicle_funds: config.config.chronicle_funds,
			networks,
			chronicles: config.chronicles,
		})
	}

	pub fn prefix(&self) -> Option<&str> {
		self.prefix.as_deref()
	}

	pub fn timechain_url(&self) -> &str {
		&self.timechain_url
	}

	pub fn chronicle_funds(&self) -> &str {
		&self.chronicle_funds
	}

	pub fn networks(&self) -> &HashMap<NetworkId, NetworkConfig> {
		&self.networks
	}

	pub fn network(&self, network: NetworkId) -> Result<&NetworkConfig> {
		self.networks.get(&network).context("no network config")
	}

	pub fn chronicles(&self) -> &[String] {
		&self.chronicles
	}

	/// Destination network gas price expressed in source network token
	pub fn gas_price(&self, src_network: NetworkId, dest_network: NetworkId) -> Result<f64> {
		let src = self.network(src_network)?;
		let dest = self.network(dest_network)?;
		Ok(gas_price(
			src.token_price_usd()?,
			src.currency_decimals,
			dest.token_price_usd()?,
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
		Ok((gas as f64 * gas_price * (1. + dest.msg_fee)) as u128)
	}

	pub fn save_prices(&mut self, prices: &HashMap<NetworkId, f64>) -> Result<()> {
		Price::write(&self.prices_path, prices)?;
		for (network, config) in &mut self.networks {
			config.token_price_usd = prices.get(network).copied();
		}
		Ok(())
	}

	pub fn save_testers(&mut self, testers: &HashMap<NetworkId, String>) -> Result<()> {
		Tester::write(&self.testers_path, testers)?;
		for (network, config) in &mut self.networks {
			config.tester_address = testers.get(network).cloned();
		}
		Ok(())
	}
}

#[derive(Clone, Debug)]
pub struct NetworkConfig {
	pub backend: Backend,
	pub proxy: Arc<[u8]>,
	pub gateway: Arc<[u8]>,
	pub tester: Arc<[u8]>,
	pub name: String,
	pub url: String,
	pub coin_id: String,
	pub currency_decimals: u8,
	pub currency_symbol: String,
	pub admin_funds: Option<String>,
	pub gateway_funds: String,
	pub chronicle_funds: String,
	pub batch_size: u32,
	pub batch_offset: u32,
	pub max_gas_price: u128,
	pub block_gas_limit: u64,
	pub shard_size: u16,
	pub shard_threshold: u16,
	pub shard_task_limit: u32,
	pub batch_gas_limit: f64,
	pub msg_gas_limit: f64,
	pub msg_fee: f64,
	pub batch_exec_gas: u64,
	pub reg_op_exec_gas: u64,
	pub unreg_op_exec_gas: u64,
	pub msg_op_exec_gas: u64,
	pub msg_session_gas: u64,
	pub msg_byte_gas: u64,
	pub token_price_usd: Option<f64>,
	pub tester_address: Option<String>,
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

	pub fn max_msg_op_gas(&self) -> u64 {
		self.msg_byte_gas * 0x6000 + self.msg_op_exec_gas + self.msg_gas_limit()
	}

	pub fn currency(&self) -> Currency {
		Currency::new(self.currency_decimals, self.currency_symbol.clone())
	}

	pub fn token_price_usd(&self) -> Result<f64> {
		self.token_price_usd.context("No token price data")
	}

	pub fn balance_to_usd(&self, balance: u128) -> Result<f64> {
		let token_price = self.token_price_usd()?;
		let decimals = self.currency_decimals;
		let factor = 10.0f64.powi(decimals as i32);
		Ok(balance as f64 / factor * token_price)
	}

	pub fn usd_to_balance(&self, usd: f64) -> Result<u128> {
		let token_price = self.token_price_usd()?;
		let decimals = self.currency_decimals;
		let factor = 10.0f64.powi(decimals as i32);
		Ok((usd / token_price * factor) as u128)
	}

	pub fn batch_gas_limit(&self) -> u64 {
		(self.block_gas_limit as f64 * self.batch_gas_limit) as u64
	}

	pub fn msg_gas_limit(&self) -> u64 {
		(self.block_gas_limit as f64 * self.msg_gas_limit) as u64
	}

	pub fn msg_fee(&self) -> Result<u64> {
		Ok(self.usd_to_balance(self.msg_fee)? as u64)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn make_sure_envs_parse() -> Result<()> {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../config/envs");
		let envs = std::fs::read_dir(&root)?;
		for env in envs {
			let env_dir = env?;
			if !env_dir.file_type()?.is_dir() {
				continue;
			}
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
				let config = Config::from_env(&env_dir.path(), &config).unwrap();
				for config in config.networks().values() {
					assert!(config.token_price_usd.is_some());
				}
			}
		}
		Ok(())
	}
}
