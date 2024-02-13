use anyhow::Result;
use chronicle::ChronicleConfig;
use clap::Parser;
use std::path::PathBuf;
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
		}
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();
	let config = ChronicleArgs::parse().config();
	let (network, network_requests) =
		chronicle::create_iroh_network(config.network_config()).await?;
	let tx_submitter = SubxtTxSubmitter::try_new(&config.timechain_url).await?;
	let subxt =
		SubxtClient::with_keyfile(&config.timechain_url, &config.timechain_keyfile, tx_submitter)
			.await?;
	chronicle::run_chronicle(config, network, network_requests, subxt).await
}
