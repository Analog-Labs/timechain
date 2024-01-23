use anyhow::Result;
use chronicle::ChronicleConfig;
use clap::Parser;
use std::path::PathBuf;
use tc_subxt::SubxtClient;
use time_primitives::NetworkId;

#[derive(Debug, Parser)]
/// workaround for https://github.com/clap-rs/clap/issues/5092
#[group(requires_all = ["network_id", "url"], multiple = true)]
pub struct ChronicleArgs {
	/// workaround for https://github.com/clap-rs/clap/issues/5092
	#[arg(required = false)]
	/// The network to be used from Analog Connector.
	#[clap(long)]
	pub network_id: NetworkId,
	/// workaround for https://github.com/clap-rs/clap/issues/5092
	#[arg(required = false)]
	/// The address of Analog Connector.
	#[clap(long)]
	pub url: String,
	/// workaround for https://github.com/clap-rs/clap/issues/5092
	#[arg(required = false)]
	/// keyfile having an account with funds for timechain.
	#[clap(long)]
	pub timechain_keyfile: PathBuf,
	/// key file for connector wallet
	#[clap(long)]
	pub keyfile: Option<PathBuf>,
	/// The timegraph url (or TIMEGTAPH_URL environment variable).
	#[clap(long)]
	pub timegraph_url: Option<String>,
	/// The timegraph session key (or TIMEGTAPH_SSK environment variable).
	#[clap(long)]
	pub timegraph_ssk: Option<String>,
	/// The secret to use for p2p networking.
	#[clap(long)]
	pub secret: Option<PathBuf>,
	/// The port to bind to for p2p networking.
	#[clap(long)]
	pub bind_port: Option<u16>,
	/// The pkarr relay for looking up nodes.
	#[clap(long)]
	pub pkarr_relay: Option<String>,
}

impl ChronicleArgs {
	pub fn config(self) -> ChronicleConfig {
		ChronicleConfig {
			network_id: self.network_id,
			url: self.url,
			timechain_keyfile: self.timechain_keyfile,
			keyfile: self.keyfile,
			timegraph_url: self.timegraph_url.or(std::env::var("TIMEGRAPH_URL").ok()),
			timegraph_ssk: self.timegraph_ssk.or(std::env::var("TIMEGRAPH_SSK").ok()),
			secret: self.secret,
			bind_port: self.bind_port,
			pkarr_relay: self.pkarr_relay,
		}
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	let config = ChronicleArgs::parse().config();
	let (_network, _network_requests) =
		chronicle::create_iroh_network(config.network_config()).await?;
	let _subxt = SubxtClient::new("ws://127.0.0.1:9944", Some(&config.timechain_keyfile)).await?;
	//chronicle::run_chronicle(config, network, network_requests, subxt).await
	Ok(())
}
