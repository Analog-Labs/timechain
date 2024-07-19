//! Make the set of bag thresholds to be used with pallet-bags-list.
use clap::{Parser, ValueEnum};
use polkadot_sdk::generate_bags::generate_thresholds;
use runtime_common::currency::TOTAL_ISSUANCE;
use std::path::PathBuf;

#[derive(Clone, Copy, ValueEnum)]
enum Chain {
	Mainnet,
	Testnet,
}

#[derive(Parser)]
// #[clap(author, version, about)]
struct Opt {
	/// How many bags to generate.
	#[arg(long, default_value_t = 200)]
	n_bags: usize,

	/// The chain to target
	chain: Chain,

	/// Where to write the output.
	output: PathBuf,
}

fn main() -> Result<(), std::io::Error> {
	let Opt { n_bags, chain, output } = Opt::parse();
	match chain {
		Chain::Mainnet => generate_thresholds::<mainnet_runtime::Runtime>(
			n_bags,
			&output,
			TOTAL_ISSUANCE,
			mainnet_runtime::ExistentialDeposit::get(),
		),
		Chain::Testnet => generate_thresholds::<testnet_runtime::Runtime>(
			n_bags,
			&output,
			TOTAL_ISSUANCE,
			testnet_runtime::EXISTENTIAL_DEPOSIT,
		),
	}
}
