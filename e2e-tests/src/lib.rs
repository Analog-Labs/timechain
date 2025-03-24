use anyhow::{Context, Result};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use tc_cli::{
	config::{ConfigYaml, ContractsConfig, GlobalConfig, NetworkConfig},
	Config, Mnemonics, NetworkId, Sender, Tc,
};
use tempfile::TempDir;
use testcontainers::{
	core::{ContainerAsync, IntoContainerPort},
	runners::AsyncRunner,
	GenericImage, ImageExt,
};
use time_primitives::{Address, GmpMessage};
use tracing_subscriber::filter::EnvFilter;

pub type Container = ContainerAsync<GenericImage>;
pub use tc_cli::Backend;

pub struct TestEnvBuilder {
	temp: TempDir,
	network: String,
	validator_name: String,
	validator: Container,
	chains: HashMap<NetworkId, Container>,
	chronicles: HashMap<NetworkId, Vec<Container>>,
	config: ConfigYaml,
	prices: HashMap<NetworkId, (String, f64)>,
}

impl TestEnvBuilder {
	pub async fn new() -> Result<Self> {
		let filter = EnvFilter::from_default_env().add_directive("info".parse()?);
		tracing_subscriber::fmt().with_env_filter(filter).try_init().ok();

		let temp = TempDir::new()?;
		let network = temp
			.path()
			.file_name()
			.unwrap()
			.to_str()
			.unwrap()
			.strip_prefix('.')
			.unwrap()
			.to_string();
		let validator_name = format!("{network}-validator");
		let validator = GenericImage::new("analoglabs/timechain-node-develop", "latest")
			.with_exposed_port(9944.tcp())
			.with_container_name(validator_name.clone())
			.with_network(network.clone())
			.with_cmd([
				"--chain=dev",
				"--base-path=/data",
				"--rpc-cors=all",
				"--rpc-methods=unsafe",
				"--unsafe-rpc-external",
				"--alice",
				"--validator",
				"--force-authoring",
				"--node-key=0000000000000000000000000000000000000000000000000000000000000001",
				"-ltxpool=trace,basic_authorship=trace,runtime=trace",
			])
			.start()
			.await?;
		let validator_host = validator.get_host().await?;
		let validator_port = validator.get_host_port_ipv4(9944).await?;
		let validator_url = format!("ws://{validator_host}:{validator_port}");
		let workspace =
			Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).parent().unwrap().to_path_buf();
		tracing::info!("workspace: {}", workspace.display());
		tracing::info!("tempdir: {}", temp.path().display());
		Ok(Self {
			temp,
			network,
			validator_name,
			validator,
			chains: Default::default(),
			chronicles: Default::default(),
			config: ConfigYaml {
				config: GlobalConfig {
					prices_path: "prices.csv".into(),
					chronicle_funds: "1.".into(),
					timechain_url: validator_url,
				},
				contracts: {
					let mut contracts = HashMap::default();
					contracts.insert(
						Backend::Evm,
						ContractsConfig {
							additional_params: workspace
								.join("gmp/evm/factory/additional_config.json"),
							proxy: workspace
								.join("analog-gmp/out/GatewayProxy.sol/GatewayProxy.json"),
							gateway: workspace.join("analog-gmp/out/Gateway.sol/Gateway.json"),
							tester: workspace.join("analog-gmp/out/GmpProxy.sol/GmpProxy.json"),
						},
					);
					contracts
				},
				networks: Default::default(),
				chronicles: Default::default(),
			},
			prices: Default::default(),
		})
	}

	pub async fn add_grpc(
		&mut self,
		network: NetworkId,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<()> {
		// add chain to docker compose
		let chain_name = format!("{}-chain-grpc-{network}", &self.network);
		let chain = GenericImage::new("analoglabs/gmp-grpc-develop", "latest")
			.with_exposed_port(3000.tcp())
			.with_container_name(&chain_name)
			.with_network(self.network.clone())
			.with_env_var("RUST_LOG", "gmp_grpc=debug,gmp_rust=debug")
			.with_env_var("RUST_BACKTRACE", "1")
			.with_cmd([format!("--network-id={network}")])
			.start()
			.await?;
		let chain_host = chain.get_host().await?;
		let chain_port = chain.get_host_port_ipv4(3000).await?;
		let chain_url = format!("http://{chain_host}:{chain_port}");
		self.chains.insert(network, chain);

		// add network config
		self.config.networks.insert(
			network,
			NetworkConfig {
				backend: Backend::Grpc,
				blockchain: "rust".into(),
				network: format!("rust-{network}"),
				url: chain_url.clone(),
				admin_funds: Some("10.".into()),
				gateway_funds: "1.".into(),
				chronicle_funds: ".1".into(),
				batch_size: 8,
				batch_offset: 0,
				batch_gas_limit: 10_000_000,
				gmp_margin: 0.,
				shard_task_limit: 50,
				route_gas_limit: 10_000_000,
				route_base_fee: 1_400_000_000,
				shard_size,
				shard_threshold,
				coin_id: 825,
				cctp_contracts: None,
				cctp_url: None,
			},
		);

		// add price data
		self.prices.insert(network, ("TT".into(), 0.01));

		// add chronicles
		for i in 0..shard_size {
			self.add_chronicle(network, Backend::Grpc, i, &format!("http://{chain_name}:3000"))
				.await?;
		}
		Ok(())
	}

	pub async fn add_evm(
		&mut self,
		network: NetworkId,
		shard_size: u16,
		shard_threshold: u16,
	) -> Result<()> {
		// add chain to docker compose
		let chain_name = format!("{}-chain-evm-{network}", &self.network);
		let chain = GenericImage::new("ghcr.io/foundry-rs/foundry", "latest")
			.with_exposed_port(8545.tcp())
			.with_container_name(&chain_name)
			.with_network(self.network.clone())
			.with_env_var("ANVIL_IP_ADDR", "0.0.0.0")
			.with_cmd([
				"anvil -b=6 --steps-tracing --order=fifo --base-fee=0 --no-request-size-limit --slots-in-an-epoch 1",
			])
			.start()
			.await?;
		let chain_host = chain.get_host().await?;
		let chain_port = chain.get_host_port_ipv4(8545).await?;
		let chain_url = format!("ws://{chain_host}:{chain_port}");
		self.chains.insert(network, chain);

		// add network config
		self.config.networks.insert(
			network,
			NetworkConfig {
				backend: Backend::Evm,
				blockchain: "anvil".into(),
				network: "dev".into(),
				url: chain_url.clone(),
				admin_funds: Some("10.".into()),
				gateway_funds: "1.".into(),
				chronicle_funds: ".1".into(),
				batch_size: 8,
				batch_offset: 0,
				batch_gas_limit: 10_000_000,
				gmp_margin: 0.,
				shard_task_limit: 50,
				route_gas_limit: 10_000_000,
				route_base_fee: 1_400_000_000,
				shard_size,
				shard_threshold,
				coin_id: 1027,
				cctp_url: Some("https://iris-api-sandbox.circle.com/attestations/".into()),
				cctp_contracts: None,
			},
		);

		// add price data
		self.prices.insert(network, ("ETH".into(), 0.01));

		// add chronicles
		for i in 0..shard_size {
			self.add_chronicle(network, Backend::Evm, i, &format!("ws://{chain_name}:8545"))
				.await?;
		}
		Ok(())
	}

	async fn add_chronicle(
		&mut self,
		network: NetworkId,
		backend: Backend,
		i: u16,
		target_url: &str,
	) -> Result<()> {
		let chronicle_name = format!("{}-chronicle-{backend}-{network}-{i}", &self.network);
		let chronicle = GenericImage::new("analoglabs/chronicle-develop", "latest")
			.with_exposed_port(8080.tcp())
			.with_container_name(chronicle_name)
			.with_network(self.network.clone())
			.with_env_var("RUST_LOG", "tc_subxt=debug,chronicle=debug,tss=debug,gmp_evm=info")
			.with_env_var("RUST_BACKTRACE", "1")
			.with_cmd([
				format!("--timechain-url=ws://{}:9944", &self.validator_name),
				format!("--target-url={target_url}"),
				format!("--backend={backend}"),
				format!("--network-id={network}"),
			])
			.start()
			.await?;
		let chronicle_host = chronicle.get_host().await?;
		let chronicle_port = chronicle.get_host_port_ipv4(8080).await?;
		let chronicle_url = format!("http://{chronicle_host}:{chronicle_port}");
		self.config.chronicles.push(chronicle_url);
		self.chronicles.entry(network).or_default().push(chronicle);
		Ok(())
	}

	pub async fn build(self) -> Result<TestEnv> {
		let env = self.temp.path().to_path_buf();
		std::fs::write(env.join("config.yaml"), serde_yaml::to_string(&self.config)?)?;
		let config = Config::new(env, self.config, self.prices);
		let tc = Tc::new(
			config,
			Mnemonics::default(),
			Sender::default(),
			self.temp.path().join("tc-cli-tx.redb"),
		)
		.await
		.context("Error creating Tc client")?;
		let testers = tc.setup_test().await?;
		Ok(TestEnv {
			_temp: self.temp,
			validator: self.validator,
			chains: self.chains,
			chronicles: self.chronicles,
			tc,
			testers,
		})
	}

	pub async fn setup(backend: Backend, shard_size: u16, shard_threshold: u16) -> Result<TestEnv> {
		let mut builder = TestEnvBuilder::new().await?;
		match backend {
			Backend::Evm => {
				builder.add_evm(0, shard_size, shard_threshold).await?;
				builder.add_evm(1, shard_size, shard_threshold).await?;
			},
			Backend::Grpc => {
				builder.add_grpc(0, shard_size, shard_threshold).await?;
				builder.add_grpc(1, shard_size, shard_threshold).await?;
			},
			Backend::Rust => {
				anyhow::bail!("unsupported backend {backend}");
			},
		}
		let tc = builder.build().await?;
		Ok(tc)
	}
}

pub struct TestEnv {
	_temp: TempDir,
	validator: Container,
	chains: HashMap<NetworkId, Container>,
	chronicles: HashMap<NetworkId, Vec<Container>>,
	tc: Tc,
	testers: HashMap<NetworkId, (Address, u64)>,
}

impl TestEnv {
	/// Returns the testers
	pub fn testers(&self) -> &HashMap<NetworkId, (Address, u64)> {
		&self.testers
	}

	/// Returns the tester.
	pub fn tester(&self, network: NetworkId) -> Result<Address> {
		Ok(self.testers.get(&network).context("missing tester")?.0)
	}

	/// Runs a smoke test
	pub async fn smoke_test(&self, payload: Vec<u8>) -> Result<GmpMessage> {
		self.exec_smoke(0, 1, &self.testers, payload).await
	}

	/// Returns the validator container
	pub fn validator_container(&self) -> &Container {
		&self.validator
	}

	/// Returns the chain container.
	pub fn chain_container(&self, network: NetworkId) -> Result<&Container> {
		self.chains.get(&network).context("no chain for network")
	}

	/// Returns the chronicle containers.
	pub fn chronicle_containers(&self, network: NetworkId) -> Result<&[Container]> {
		Ok(self.chronicles.get(&network).context("no chronicles for network")?.as_slice())
	}
}

impl Deref for TestEnv {
	type Target = Tc;

	fn deref(&self) -> &Self::Target {
		&self.tc
	}
}

impl DerefMut for TestEnv {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.tc
	}
}
