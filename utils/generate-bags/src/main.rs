//! Make the set of bag thresholds to be used with pallet-bags-list.
use clap::Parser;
use polkadot_sdk::generate_bags::generate_thresholds;
use std::path::PathBuf;
use timechain_runtime::{currency::TOTAL_ISSUANCE, ExistentialDeposit, Runtime};

#[derive(Parser)]
// #[clap(author, version, about)]
struct Opt {
	/// How many bags to generate.
	#[arg(long, default_value_t = 200)]
	n_bags: usize,

	/// Where to write the output.
	output: PathBuf,
}

fn main() -> Result<(), std::io::Error> {
	let Opt { n_bags, output } = Opt::parse();
	generate_thresholds::<Runtime>(n_bags, &output, TOTAL_ISSUANCE, ExistentialDeposit::get())
}
