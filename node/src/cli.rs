use std::path::PathBuf;

use time_primitives::NetworkId;

#[derive(Debug, clap::Parser)]
#[group(skip)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, clap::Parser)]
#[group(skip)]
pub struct RunCmd {
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,
	#[clap(flatten)]
	pub chronicle: Option<ChronicleArgs>,
}

#[derive(Debug, clap::Parser)]
/// workaround for https://github.com/clap-rs/clap/issues/5092
#[group(requires_all = ["network_id", "target_url", "target_keyfile", "timechain_keyfile"], multiple = true)]
pub struct ChronicleArgs {
	/// The network to be used from Analog Connector.
	#[arg(required = false)]
	#[clap(long)]
	pub network_id: NetworkId,
	/// The secret to use for p2p networking.
	#[clap(long)]
	pub network_keyfile: Option<PathBuf>,
	/// The port to bind to for p2p networking.
	#[clap(long)]
	pub bind_port: Option<u16>,
	/// Enables iroh networking.
	#[clap(long)]
	pub enable_iroh: bool,
	/// The address of Analog Connector.
	#[arg(required = false)]
	#[clap(long)]
	pub target_url: String,
	/// key file for connector wallet
	#[arg(required = false)]
	#[clap(long)]
	pub target_keyfile: PathBuf,
	/// keyfile having an account with funds for timechain.
	#[arg(required = false)]
	#[clap(long)]
	pub timechain_keyfile: PathBuf,
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
