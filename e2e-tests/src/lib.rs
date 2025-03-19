use anyhow::{Context, Result};
use docker_compose_types::{Command as ServiceCommand, Compose, Environment, Service};
use indexmap::IndexMap;
use rusty_docker_compose::DockerCompose;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::Command;
use tc_cli::{
	config::{ConfigYaml, ContractsConfig, GlobalConfig, NetworkConfig},
	Backend, Config, Mnemonics, NetworkId, Sender, Tc,
};
use tempfile::TempDir;
use tracing_subscriber::filter::EnvFilter;

pub struct TestEnvBuilder {
	compose: Compose,
	config: ConfigYaml,
	prices: HashMap<NetworkId, (String, f64)>,
}

impl TestEnvBuilder {
	pub fn new() -> Self {
		Self {
			compose: {
				let mut service = Service::default();
				service.image = Some("analoglabs/timechain-node-develop".into());
				service.command = Some(ServiceCommand::Args(vec![
					"--chain=dev".into(),
					"--base-path=/data".into(),
					"--rpc-cors=all".into(),
					"--rpc-methods=unsafe".into(),
					"--alice".into(),
					"--validator".into(),
					"--force-authoring".into(),
					"--node-key=0000000000000000000000000000000000000000000000000000000000000001"
						.into(),
					"-ltxpool=trace,basic_authorship=trace,runtime=trace".into(),
				]));
				let mut compose = Compose::default();
				compose.services.0.insert("validator".into(), Some(service));
				compose
			},
			config: ConfigYaml {
				config: GlobalConfig {
					prices_path: "prices.csv".into(),
					chronicle_funds: "1.".into(),
					timechain_url: "ws://validator:9944".into(),
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
		}
	}

	pub fn add_grpc(&mut self, network: NetworkId, shard_size: u16, shard_threshold: u16) {
		let chain_name = format!("chain-grpc-{network}");
		let chain_url = format!("http://{chain_name}:3000");

		// add chain to docker compose
		let mut chain = Service::default();
		chain.image = Some("analoglabs/gmp-grpc-develop".into());
		chain.command = Some(ServiceCommand::Args(vec![format!("--network-id={network}")]));
		let mut env = IndexMap::default();
		env.insert("RUST_LOG".into(), Some("gmp_grpc=debug,gmp_rust=debug".into()));
		env.insert("RUST_BACKTRACE".into(), Some("1".into()));
		chain.environment = Environment::KvPair(env);
		self.compose.services.0.insert(chain_name.clone(), Some(chain));

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
			self.add_chronicle(network, Backend::Grpc, i, &chain_url);
		}
	}

	pub fn add_evm(&mut self, network: NetworkId, shard_size: u16, shard_threshold: u16) {
		let chain_name = format!("chain-evm-{network}");
		let chain_url = format!("ws://{chain_name}:8454");

		// add chain to docker compose
		let mut chain = Service::default();
		chain.image = Some("ghcr.io/foundry-rs/foundry:latest".into());
		chain.command = Some(ServiceCommand::Args(vec![
			"anvil".into(),
			"-b=2".into(),
			"--steps-tracing".into(),
			"--order=fifo".into(),
			"--base-fee=0".into(),
			"--no-request-size-limit".into(),
		]));
		let mut env = IndexMap::default();
		env.insert("ANVIL_IP_ADDR".into(), Some("0.0.0.0".into()));
		chain.environment = Environment::KvPair(env);

		// add network config
		self.config.networks.insert(
			network,
			NetworkConfig {
				backend: Backend::Evm,
				blockchain: "anvil".into(),
				network: format!("anvil-{network}"),
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
			self.add_chronicle(network, Backend::Evm, i, &chain_url);
		}
	}

	fn add_chronicle(&mut self, network: NetworkId, backend: Backend, i: u16, target_url: &str) {
		let mut chronicle = Service::default();
		chronicle.image = Some("analoglabs/chronicle-develop".into());
		chronicle.command = Some(ServiceCommand::Args(vec![
			"--timechain-url=ws://validator:9944".into(),
			format!("--target-url={target_url}"),
			format!("--backend={backend}"),
			format!("--network-id={network}"),
		]));
		let mut env = IndexMap::default();
		env.insert(
			"RUST_LOG".into(),
			Some("tc_subxt=debug,chronicle=debug,tss=debug,gmp_evm=info".into()),
		);
		chronicle.environment = Environment::KvPair(env);
		let service_name = format!("chronicle-{backend}-{network}-{i}");
		self.compose.services.0.insert(service_name, Some(chronicle));
		self.config.chronicles.push(format!("http://{service_name}:8080"));
	}

	pub async fn build(self) -> Result<TestEnv> {
		let temp = TempDir::new()?;
		let compose = serde_yaml::to_string(&self.compose)?;
		let compose_path = temp.path().join("docker-compose.yml");
		std::fs::write(compose_path, compose)?;
		let config = Config::new(temp.path(), self.config, self.prices);
		TestEnv::new(temp, compose_path, config).await
	}
}

pub struct TestEnv {
	temp: TempDir,
	docker: DockerCompose,
	compose_path: PathBuf,
	tc: Tc,
}

impl TestEnv {
	async fn new(temp: TempDir, compose_path: PathBuf, config: Config) -> Result<Self> {
		let filter = EnvFilter::from_default_env().add_directive("info".parse()?);
		tracing_subscriber::fmt().with_env_filter(filter).try_init().ok();

		let docker = DockerCompose::new(compose_path, temp.path());
		let tc = Tc::new(
			config,
			Mnemonics::default(),
			Sender::default(),
			temp.path().join("tc-cli-tx.redb"),
		)
		.await
		.context("Error creating Tc client")?;

		Ok(TestEnv { temp, docker, compose_path, tc })
	}

	/// Restarts the containers
	pub fn restart(&self, containers: &[&str]) -> Result<()> {
		docker_restart(&self.compose_path, containers)
	}
}

impl Deref for TestEnv {
	type Target = Tc;

	fn deref(&self) -> &Self::Target {
		&self.tc
	}
}

fn docker_restart(path: &Path, containers: &[&str]) -> Result<()> {
	let status = Command::new("docker")
		.arg("compose")
		.arg("-f")
		.arg(path)
		.arg("stop")
		.args(containers)
		.status()
		.context("failed to stop containers")?;
	if !status.success() {
		anyhow::bail!("stopping containers returned status {status}");
	}
	let status = Command::new("docker")
		.arg("compose")
		.arg("-f")
		.arg(path)
		.arg("start")
		.args(containers)
		.status()
		.context("failed to start containers")?;
	if !status.success() {
		anyhow::bail!("starting containers returned status {status}");
	}
	Ok(())
}
