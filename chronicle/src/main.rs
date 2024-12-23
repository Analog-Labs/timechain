use anyhow::{Context, Result};
use bip39::Mnemonic;
use chronicle::ChronicleConfig;
use clap::Parser;
use futures::channel::mpsc;
use futures::FutureExt;
use gmp::Backend;
use std::sync::Arc;
use std::{
	path::{Path, PathBuf},
	time::Duration,
};
use tc_subxt::SubxtClient;
use time_primitives::NetworkId;

#[cfg(not(feature = "testnet"))]
compile_error!("GMP is currently not supported on mainnet.");

#[derive(Debug, Parser)]
pub struct ChronicleArgs {
	/// The network to be used from Analog Connector.
	#[clap(long)]
	pub network_id: NetworkId,
	/// The secret to use for p2p networking.
	#[clap(long)]
	pub network_keyfile: PathBuf,
	/// The address of target chain rpc.
	#[clap(long)]
	pub target_url: String,
	/// key file for connector wallet
	#[clap(long)]
	pub target_keyfile: PathBuf,
	/// target min balance
	#[clap(long, default_value_t = 0)]
	pub target_min_balance: u128,
	/// timechain min balance
	#[clap(long, default_value_t = 0)]
	pub timechain_min_balance: u128,
	/// Url for timechain node to connect to.
	#[clap(long)]
	pub timechain_url: String,
	/// keyfile having an account with funds for timechain.
	#[clap(long)]
	pub timechain_keyfile: PathBuf,
	/// Enables Prometheus exported metrics
	#[clap(long, default_value_t = true)]
	pub prometheus_enabled: bool,
	/// Port for exporting Prometheus metrics
	#[clap(long, default_value_t = 9090)]
	pub prometheus_port: u16,
	/// Location to cache tss keyshares.
	#[clap(long, default_value = "/tmp")]
	pub tss_keyshare_cache: PathBuf,
	/// Gmp backend to use.
	#[clap(long, default_value = "evm")]
	pub backend: Backend,
}

impl ChronicleArgs {
	fn config(self, network_key: [u8; 32], target_mnemonic: String) -> Result<ChronicleConfig> {
		Ok(ChronicleConfig {
			network_id: self.network_id,
			network_key,
			timechain_min_balance: self.timechain_min_balance,
			target_min_balance: self.target_min_balance,
			target_url: self.target_url,
			target_mnemonic,
			tss_keyshare_cache: self.tss_keyshare_cache,
			backend: self.backend,
		})
	}
}

fn generate_key(path: &Path) -> Result<()> {
	let mut seed = [0; 32];
	getrandom::getrandom(&mut seed)?;
	let mnemonic = Mnemonic::from_entropy(&seed)?;
	std::fs::write(path, mnemonic.to_string())?;
	Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.with_file(true)
		.with_line_number(true)
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.init();
	std::panic::set_hook(Box::new(tracing_panic::panic_hook));

	let args = ChronicleArgs::parse();

	if !args.tss_keyshare_cache.exists() {
		std::fs::create_dir_all(&args.tss_keyshare_cache)?;
	}

	if !args.timechain_keyfile.exists() {
		generate_key(&args.timechain_keyfile)?;
	}

	if !args.target_keyfile.exists() {
		generate_key(&args.target_keyfile)?;
	}

	if !args.network_keyfile.exists() {
		let mut secret = [0; 32];
		getrandom::getrandom(&mut secret)?;
		std::fs::write(&args.network_keyfile, secret)?;
	}

	let timechain_mnemonic = std::fs::read_to_string(&args.timechain_keyfile)
		.context("failed to read timechain keyfile")?;
	let target_mnemonic =
		std::fs::read_to_string(&args.target_keyfile).context("failed to read target keyfile")?;
	let network_key = std::fs::read(&args.network_keyfile)
		.context("network keyfile doesn't exist")?
		.try_into()
		.map_err(|_| anyhow::anyhow!("invalid secret"))?;

	// Setup Prometheus exporter if enabled
	if args.prometheus_enabled {
		let binding = format!("0.0.0.0:{}", args.prometheus_port).parse().unwrap();
		if let Err(err) = prometheus_exporter::start(binding) {
			panic!("Error while starting Prometheus exporter: {}", err);
		}
	}

	loop {
		if SubxtClient::get_client(&args.timechain_url).await.is_ok() {
			break;
		} else {
			tracing::error!("Error connecting to {} retrying", &args.timechain_url);
			tokio::time::sleep(Duration::from_secs(5)).await;
		}
	}

	let subxt = SubxtClient::with_key(&args.timechain_url, &timechain_mnemonic).await?;

	let config = args.config(network_key, target_mnemonic)?;

	let (tx, rx) = mpsc::channel(1);
	let admin = chronicle::admin::listen(8080, rx);
	let chronicle = chronicle::run_chronicle(config, Arc::new(subxt), tx);
	let signal = shutdown_signal();

	futures::select! {
		_ = chronicle.fuse() => {}
		_ = signal.fuse() => {}
		_ = admin.fuse() => {}
	};

	std::process::exit(0);
}

async fn shutdown_signal() {
	let ctrl_c = async {
		tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
	};

	#[cfg(unix)]
	let terminate = async {
		tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
			.expect("failed to install signal handler")
			.recv()
			.await;
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		_ = ctrl_c => {},
		_ = terminate => {},
	}
}
