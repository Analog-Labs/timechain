use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use tc_cli::{
	config::{BackendConfig, ConfigYaml, GlobalConfig, NetworkConfig},
	NetworkId, Sender, Tc,
};
use tempfile::TempDir;
use testcontainers::{
	core::{ContainerAsync, IntoContainerPort, Mount},
	runners::AsyncRunner,
	GenericImage, ImageExt,
};
use time_primitives::{Address32, GmpMessage};
use tracing_subscriber::filter::EnvFilter;
use zstd::{Decoder, Encoder};

pub type Container = ContainerAsync<GenericImage>;
pub use tc_cli::Backend;

fn try_init_logger() {
	let filter = EnvFilter::from_default_env().add_directive("info".parse().unwrap());
	tracing_subscriber::fmt().with_env_filter(filter).try_init().ok();
}

pub struct TestEnvBuilder {
	temp: TempDir,
	network: String,
	validator_name: String,
	validator: Container,
	chains: HashMap<NetworkId, Container>,
	chronicles: HashMap<NetworkId, Vec<Container>>,
	config: ConfigYaml,
	prices: HashMap<NetworkId, (String, f64)>,
	snapshot: PathBuf,
}

impl TestEnvBuilder {
	pub async fn new(snapshot: PathBuf) -> Result<Self> {
		try_init_logger();
		let temp = TempDir::new()?;
		if snapshot.exists() {
			unarchive(&snapshot, temp.path())?;
		}
		let network = temp
			.path()
			.file_name()
			.unwrap()
			.to_str()
			.unwrap()
			.strip_prefix('.')
			.unwrap()
			.to_string();
		let workspace =
			Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).parent().unwrap().to_path_buf();
		tracing::info!("workspace: {}", workspace.display());
		tracing::info!("tempdir: {}", temp.path().display());

		let validator_port = pick_free_port()?;
		let validator_name = format!("{network}-validator");
		let validator_mount = temp.path().join("tc");
		std::fs::create_dir_all(&validator_mount)?;
		let validator = GenericImage::new("analoglabs/timechain-node-develop", "latest")
			.with_exposed_port(9944.tcp())
			.with_mapped_port(validator_port, testcontainers::core::ContainerPort::Tcp(9944))
			.with_container_name(validator_name.clone())
			.with_network(network.clone())
			.with_cmd([
				"--chain=dev",
				"--base-path=/state",
				"--rpc-cors=all",
				"--rpc-methods=unsafe",
				"--unsafe-rpc-external",
				"--alice",
				"--validator",
				"--force-authoring",
				"--node-key=0000000000000000000000000000000000000000000000000000000000000001",
				"-ltxpool=trace,basic_authorship=trace,runtime=trace",
			])
			.with_mount(Mount::bind_mount(validator_mount.to_str().unwrap(), "/state"))
			.start()
			.await?;
		let validator_host = validator.get_host().await?;
		let validator_url = format!("ws://{validator_host}:{validator_port}");
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
				backends: {
					let mut backends = HashMap::default();
					backends.insert(
						Backend::Evm,
						BackendConfig {
							chain_dict: workspace.join("gmp/evm/auxiliary/chains.json"),
							factory: workspace.join("gmp/evm/auxiliary/factory.json"),
							proxy: workspace
								.join("analog-gmp/out/GatewayProxy.sol/GatewayProxy.json"),
							gateway: workspace.join("analog-gmp/out/Gateway.sol/Gateway.json"),
							tester: workspace.join("analog-gmp/out/GmpProxy.sol/GmpProxy.json"),
						},
					);
					backends
				},
				networks: Default::default(),
				chronicles: Default::default(),
			},
			prices: Default::default(),
			snapshot,
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
		let chain_mount = self.temp.path().join(&chain_name);
		std::fs::create_dir_all(&chain_mount)?;
		let chain_name = format!("{}-{chain_name}", &self.network);
		let chain = GenericImage::new("analoglabs/gmp-grpc-develop", "latest")
			.with_exposed_port(3000.tcp())
			.with_container_name(&chain_name)
			.with_network(self.network.clone())
			.with_env_var("RUST_LOG", "gmp_grpc=debug,gmp_rust=debug")
			.with_env_var("RUST_BACKTRACE", "1")
			.with_cmd([format!("--network-id={network}"), "--db=/state/grpc".into()])
			.with_mount(Mount::bind_mount(chain_mount.to_str().unwrap(), "/state"))
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
				name: format!("grpc-{network}"),
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
		let chain_name = format!("chain-evm-{network}");
		let chain_mount = self.temp.path().join(&chain_name);
		std::fs::create_dir_all(&chain_mount)?;
		let chain_name = format!("{}-{chain_name}", &self.network);
		let chain = GenericImage::new("ghcr.io/foundry-rs/foundry", "latest")
			.with_exposed_port(8545.tcp())
			.with_container_name(&chain_name)
			.with_network(self.network.clone())
			.with_env_var("ANVIL_IP_ADDR", "0.0.0.0")
			.with_cmd([
				"anvil -b=6 --steps-tracing --order=fifo --base-fee=0 --no-request-size-limit --slots-in-an-epoch 1 --state /state/anvil",
			])
			.with_mount(Mount::bind_mount(chain_mount.to_str().unwrap(),"/state"))
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
				name: format!("evm-{network}"),
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
		let chronicle_name = format!("chronicle-{backend}-{network}-{i}");
		let chronicle_mount = self.temp.path().join(&chronicle_name);
		std::fs::create_dir_all(&chronicle_mount)?;
		let chronicle_name = format!("{}-{chronicle_name}", &self.network);
		let mut cmd = vec![
			format!("--timechain-url=ws://{}:9944", &self.validator_name),
			format!("--target-url={target_url}"),
			format!("--backend={backend}"),
			format!("--network-id={network}"),
			format!("--network-keyfile=/state/network_keyfile"),
			format!("--target-keyfile=/state/target_keyfile"),
			format!("--timechain-keyfile=/state/timechain_keyfile"),
			format!("--tx-db=/state/tx-db"),
			format!("--tss-keyshare-cache=/state/tss"),
		];
		if backend == Backend::Evm {
			cmd.push("--chain-dict=/etc/chains.json".to_string());
		}
		let chronicle = GenericImage::new("analoglabs/chronicle-develop", "latest")
			.with_exposed_port(8080.tcp())
			.with_container_name(chronicle_name)
			.with_network(self.network.clone())
			.with_env_var("RUST_LOG", "tc_subxt=debug,chronicle=debug,tss=debug,gmp_evm=info")
			.with_env_var("RUST_BACKTRACE", "1")
			.with_cmd(cmd)
			.with_mount(Mount::bind_mount(chronicle_mount.to_str().unwrap(), "/state"))
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
		tc_cli::config::write_prices(&env.join("prices.csv"), &self.prices)?;
		std::env::set_var("TC_CLI_ENV", &env);
		Ok(TestEnv {
			temp: self.temp,
			validator: self.validator,
			chains: self.chains,
			chronicles: self.chronicles,
			snapshot: self.snapshot,
		})
	}
}

pub struct TestEnv {
	temp: TempDir,
	validator: Container,
	chains: HashMap<NetworkId, Container>,
	chronicles: HashMap<NetworkId, Vec<Container>>,
	snapshot: PathBuf,
}

impl TestEnv {
	/// Creates a new test environment.
	pub async fn new(backend: Backend, tss: bool) -> Result<(Self, Tester)> {
		let mut snapshot = backend.to_string();
		if tss {
			snapshot.push_str("-tss");
		}
		snapshot.push_str(".tar.zst");
		let snapshot_path = std::env::temp_dir().join(snapshot);
		let (shard_size, shard_threshold) = if tss { (2, 2) } else { (1, 1) };
		let mut builder = TestEnvBuilder::new(snapshot_path).await?;
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
		let env = builder.build().await?;
		let tc = Tester::new().await?;
		env.snapshot().await?;
		Ok((env, tc))
	}

	pub fn env(&self) -> &Path {
		self.temp.path()
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

	async fn snapshot(&self) -> Result<()> {
		if self.snapshot.exists() {
			return Ok(());
		}

		for (_, chronicles) in &self.chronicles {
			for chronicle in chronicles {
				chronicle.stop().await?;
			}
		}
		for (_, chain) in &self.chains {
			chain.stop().await?;
		}
		self.validator.stop().await?;

		archive(self.env(), &self.snapshot)?;

		self.validator.start().await?;
		for (_, chain) in &self.chains {
			chain.start().await?;
		}
		for (_, chronicles) in &self.chronicles {
			for chronicle in chronicles {
				chronicle.start().await?;
			}
		}

		Ok(())
	}
}

fn archive(dir: &Path, file: &Path) -> Result<()> {
	let mut ar = Builder::new(Encoder::new(File::create(file)?, 13)?);
	ar.append_dir_all("", dir)?;
	ar.into_inner()?.finish()?;
	Ok(())
}

fn unarchive(file: &Path, dir: &Path) -> Result<()> {
	Archive::new(Decoder::new(BufReader::new(File::open(file)?))?).unpack(dir)?;
	Ok(())
}

pub struct Tester {
	tc: Tc,
	testers: HashMap<NetworkId, (Address32, u64)>,
}

impl Tester {
	pub async fn new() -> Result<Self> {
		try_init_logger();
		let env = std::env::var("TC_CLI_ENV").context("TC_CLI_ENV not set")?;
		let env = Path::new(&env).to_path_buf();
		let tc =
			Tc::from_env(env.clone(), "config.yaml", Sender::default(), env.join("tc-cli-tx.redb"))
				.await
				.context("Error creating Tc client")?;
		let testers = tc.setup_test().await?;
		Ok(Self { tc, testers })
	}

	/// Returns the testers
	pub fn testers(&self) -> &HashMap<NetworkId, (Address32, u64)> {
		&self.testers
	}

	/// Returns the tester.
	pub fn tester(&self, network: NetworkId) -> Result<Address32> {
		Ok(self.testers.get(&network).context("missing tester")?.0)
	}

	/// Runs a smoke test
	pub async fn smoke_test(&self, payload: Vec<u8>) -> Result<GmpMessage> {
		self.tc.exec_smoke(0, 1, &self.testers, payload).await
	}
}

impl Deref for Tester {
	type Target = Tc;
	fn deref(&self) -> &Self::Target {
		&self.tc
	}
}

impl DerefMut for Tester {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.tc
	}
}

fn pick_free_port() -> Result<u16> {
	let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
	let port = listener.local_addr()?.port();
	Ok(port)
}
