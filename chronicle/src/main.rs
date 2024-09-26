use anyhow::{Context, Result};
use bip39::Mnemonic;
use chronicle::ChronicleConfig;
use clap::Parser;
use std::{
	path::{Path, PathBuf},
	time::Duration,
};
use tc_subxt::{MetadataVariant, SubxtClient, SubxtTxSubmitter};
use time_primitives::NetworkId;

#[derive(Debug, Parser)]
pub struct ChronicleArgs {
	/// The network to be used from Analog Connector.
	#[clap(long)]
	pub network_id: NetworkId,
	/// The secret to use for p2p networking.
	#[clap(long)]
	pub network_keyfile: PathBuf,
	/// The port to bind to for p2p networking.
	#[clap(long)]
	pub network_port: Option<u16>,
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
	/// Metadata version to use to connect to timechain node.
	#[clap(long)]
	pub timechain_metadata: Option<MetadataVariant>,
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
}

impl ChronicleArgs {
	fn config(self, network_key: [u8; 32], target_mnemonic: String) -> Result<ChronicleConfig> {
		Ok(ChronicleConfig {
			network_id: self.network_id,
			network_key,
			network_port: self.network_port,
			timechain_min_balance: self.timechain_min_balance,
			target_min_balance: self.target_min_balance,
			target_url: self.target_url,
			target_mnemonic,
			tss_keyshare_cache: self.tss_keyshare_cache,
			admin: true,
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

	let tx_submitter = loop {
		if let Ok(t) = SubxtTxSubmitter::try_new(&args.timechain_url).await {
			break t;
		} else {
			tracing::error!("Error connecting to {} retrying", &args.timechain_url);
			tokio::time::sleep(Duration::from_secs(5)).await;
		}
	};

	let subxt = SubxtClient::with_key(
		&args.timechain_url,
		args.timechain_metadata.unwrap_or_default(),
		&timechain_mnemonic,
		tx_submitter,
	)
	.await?;
	chronicle::run_chronicle::<gmp_grpc::Connector>(
		args.config(network_key, target_mnemonic)?,
		subxt,
	)
	.await
}
