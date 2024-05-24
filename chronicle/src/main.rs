use anyhow::Result;
use chronicle::ChronicleConfig;
use clap::Parser;
use std::{path::PathBuf, time::Duration};
use tc_subxt::{SubxtClient, SubxtTxSubmitter};
use time_primitives::NetworkId;

#[derive(Debug, Parser)]
pub struct ChronicleArgs {
	/// The network to be used from Analog Connector.
	#[clap(long)]
	pub network_id: NetworkId,
	/// The secret to use for p2p networking.
	#[clap(long)]
	pub network_keyfile: Option<PathBuf>,
	/// The port to bind to for p2p networking.
	#[clap(long)]
	pub network_port: Option<u16>,
	/// The address of target chain rpc.
	#[clap(long)]
	pub target_url: String,
	/// key file for connector wallet
	#[clap(long)]
	pub target_keyfile: PathBuf,
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
	pub fn config(self) -> ChronicleConfig {
		ChronicleConfig {
			network_id: self.network_id,
			network_keyfile: self.network_keyfile,
			network_port: self.network_port,
			timechain_url: self.timechain_url,
			timechain_keyfile: self.timechain_keyfile,
			target_url: self.target_url,
			target_keyfile: self.target_keyfile,
			tss_keyshare_cache: self.tss_keyshare_cache,
		}
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
		.init();
	std::panic::set_hook(Box::new(tracing_panic::panic_hook));

	let args = ChronicleArgs::parse();

	if !args.tss_keyshare_cache.exists() {
		anyhow::bail!("tss keyshare cache doesn't exist");
	}
	// Setup Prometheus exporter if enabled
	if args.prometheus_enabled {
		let binding = format!("0.0.0.0:{}", args.prometheus_port).parse().unwrap();
		if let Err(err) = prometheus_exporter::start(binding) {
			panic!("Error while starting Prometheus exporter: {}", err);
		}
	}

	let config = args.config();

	let (network, network_requests) =
		chronicle::create_iroh_network(config.network_config()).await?;
	let tx_submitter = loop {
		if let Ok(t) = SubxtTxSubmitter::try_new(&config.timechain_url).await {
			break t;
		} else {
			tracing::error!("Error connecting to {} retrying", &config.timechain_url);
			tokio::time::sleep(Duration::from_secs(5)).await;
		}
	};

	let subxt =
		SubxtClient::with_keyfile(&config.timechain_url, &config.timechain_keyfile, tx_submitter)
			.await?;
	chronicle::run_chronicle(config, network, network_requests, subxt).await
}
