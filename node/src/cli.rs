use std::path::PathBuf;

use time_primitives::NetworkId;

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, clap::Parser)]
pub struct RunCmd {
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,
	#[clap(flatten)]
	pub chronicle: Option<ChronicleArgs>,
}

#[derive(Debug, clap::Parser)]
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
	/// Enables iroh networking.
	#[clap(long)]
	pub enable_iroh: bool,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management cli utilities
	#[clap(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[clap(subcommand)]
	Benchmark(Box<frame_benchmarking_cli::BenchmarkCmd>),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}
