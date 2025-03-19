use anyhow::{Context, Result};
use std::collections::HashMap;
use std::ops::Deref;
use tc_cli::{
	config::{ConfigYaml, ContractsConfig, GlobalConfig, NetworkConfig},
	Backend, Config, Mnemonics, NetworkId, Sender, Tc,
};
use tempfile::TempDir;
use testcontainers::{
	core::wait::WaitFor,
	core::{ContainerAsync, IntoContainerPort},
	runners::AsyncRunner,
	GenericImage, ImageExt,
};
use tracing_subscriber::filter::EnvFilter;

pub type Container = ContainerAsync<GenericImage>;

pub struct TestEnvBuilder {
	temp: TempDir,
	network: String,
	validator: Container,
	chains: HashMap<NetworkId, Container>,
	chronicles: HashMap<NetworkId, Vec<Container>>,
	config: ConfigYaml,
	prices: HashMap<NetworkId, (String, f64)>,
}

impl TestEnvBuilder {
	pub async fn new() -> Result<Self> {
		let temp = TempDir::new()?;
		let network = temp.path().file_name().unwrap().to_str().unwrap().to_string();
		let validator = GenericImage::new("analoglabs/timechain-node-develop", "latest")
			.with_exposed_port(9944.tcp())
			.with_wait_for(WaitFor::message_on_stderr("Idle"))
			.with_container_name("validator")
			.with_network(network.clone())
			.with_cmd([
				"--chain=dev",
				"--base-path=/data",
				"--rpc-cors=all",
				"--rpc-methods=unsafe",
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
		Ok(Self {
			temp,
			network,
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
							additional_params: "factory/additional_config.json".into(),
							proxy: "contracts/GatewayProxy.sol/GatewayProxy.json".into(),
							gateway: "contracts/Gateway.sol/Gateway.json".into(),
							tester: "contracts/GmpProxy.sol/GmpProxy.json".into(),
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
		let chain_name = format!("chain-grpc-{network}");
		let chain = GenericImage::new("analoglabs/gmp-grpc-develop", "latest")
			.with_exposed_port(3000.tcp())
			.with_container_name(chain_name)
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
				batch_size: 64,
				batch_offset: 0,
				batch_gas_limit: 10_000_000,
				gmp_margin: 0.,
				shard_task_limit: 50,
				route_gas_limit: 10_000_000,
				route_base_fee: 0,
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
			self.add_chronicle(network, Backend::Grpc, i, &chain_url).await?;
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
		let chain_name = format!("chain-evm-{network}");
		let chain = GenericImage::new("ghcr.io/foundry-rs/foundry", "latest")
			.with_exposed_port(8454.tcp())
			.with_wait_for(WaitFor::message_on_stdout("Block Number:"))
			.with_container_name(chain_name)
			.with_network(self.network.clone())
			.with_env_var("ANVIL_IP_ADDR", "0.0.0.0")
			.with_cmd([
				"anvil",
				"-b=2",
				"--steps-tracing",
				"--order=fifo",
				"--base-fee=0",
				"--no-request-size-limit",
			])
			.start()
			.await?;
		let chain_host = chain.get_host().await?;
		let chain_port = chain.get_host_port_ipv4(8454).await?;
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
				batch_size: 64,
				batch_offset: 0,
				batch_gas_limit: 10_000_000,
				gmp_margin: 0.,
				shard_task_limit: 50,
				route_gas_limit: 10_000_000,
				route_base_fee: 0,
				shard_size,
				shard_threshold,
				coin_id: 1027,
				cctp_url: None,
				cctp_contracts: None,
			},
		);

		// add price data
		self.prices.insert(network, ("ETH".into(), 1950.0));

		// add chronicles
		for i in 0..shard_size {
			self.add_chronicle(network, Backend::Evm, i, &chain_url).await?;
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
		let chronicle_name = format!("chronicle-{backend}-{network}-{i}");
		let chronicle = GenericImage::new("analoglabs/chronicle-develop", "latest")
			.with_exposed_port(8080.tcp())
			.with_container_name(chronicle_name)
			.with_network(self.network.clone())
			.with_env_var("RUST_LOG", "tc_subxt=debug,chronicle=debug,tss=debug,gmp_evm=info")
			.with_env_var("RUST_BACKTRACE", "1")
			.with_cmd([
				format!("--timechain-url={}", &self.config.config.timechain_url),
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
		let filter = EnvFilter::from_default_env().add_directive("info".parse()?);
		tracing_subscriber::fmt().with_env_filter(filter).try_init().ok();
		let config = Config::new(self.temp.path().into(), self.config, self.prices);
		let tc = Tc::new(
			config,
			Mnemonics::default(),
			Sender::default(),
			self.temp.path().join("tc-cli-tx.redb"),
		)
		.await
		.context("Error creating Tc client")?;
		Ok(TestEnv {
			_temp: self.temp,
			validator: self.validator,
			chains: self.chains,
			chronicles: self.chronicles,
			tc,
		})
	}
}

pub struct TestEnv {
	_temp: TempDir,
	validator: Container,
	chains: HashMap<NetworkId, Container>,
	chronicles: HashMap<NetworkId, Vec<Container>>,
	tc: Tc,
}

impl TestEnv {
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
