use anyhow::Result;
use chronicle::ChronicleConfig;
use clap::Parser;
use std::path::PathBuf;
use tc_subxt::{SubxtClient, SubxtTxSubmitter};
use time_primitives::NetworkId;

#[derive(Debug, Parser)]
/// workaround for https://github.com/clap-rs/clap/issues/5092
#[group(requires_all = ["network_id", "url"], multiple = true)]
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
	/// The address of Analog Connector.
	#[clap(long)]
	pub target_url: String,
	/// key file for connector wallet
	#[clap(long)]
	pub target_keyfile: PathBuf,
	/// keyfile having an account with funds for timechain.
	#[clap(long)]
	pub timechain_url: String,
	/// keyfile having an account with funds for timechain.
	#[clap(long)]
	pub timechain_keyfile: PathBuf,
	/// The timegraph url (or TIMEGTAPH_URL environment variable).
	#[clap(long)]
	pub timegraph_url: Option<String>,
	/// The timegraph session key (or TIMEGTAPH_SSK environment variable).
	#[clap(long)]
	pub timegraph_ssk: Option<String>,
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
			timegraph_url: self.timegraph_url.or(std::env::var("TIMEGRAPH_URL").ok()),
			timegraph_ssk: self.timegraph_ssk.or(std::env::var("TIMEGRAPH_SSK").ok()),
		}
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let config = ChronicleArgs::parse().config();
	let (network, network_requests) =
		chronicle::create_iroh_network(config.network_config()).await?;
	let url = "ws://127.0.0.1:9944";
	let tx_submitter = SubxtTxSubmitter::try_new(url).await?;
	let subxt = SubxtClient::with_keyfile(url, &config.timechain_keyfile, tx_submitter).await?;
	chronicle::run_chronicle(config, network, network_requests, subxt).await
}
