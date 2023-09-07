#[derive(Debug, clap::Parser)]
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
pub struct ChronicleArgs {
	/// The chain used by Analog Connector.
	#[clap(long)]
	pub blockchain: time_primitives::Network,
	/// The network to be used from Analog Connector.
	#[clap(long)]
	pub network: String,
	/// The address of Analog Connector.
	#[clap(long)]
	pub url: String,
	/// key file for connector wallet
	#[clap(long)]
	pub keyfile: Option<String>,
	/// The timegraph url (or TIMEGTAPH_URL environment variable).
	#[clap(long)]
	pub timegraph_url: Option<String>,
	/// The timegraph session key (or TIMEGTAPH_SSK environment variable).
	#[clap(long)]
	pub timegraph_ssk: Option<String>,
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
