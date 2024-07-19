//! Substrate Node CLI

mod chain_spec;
#[macro_use]
mod service;
//mod benchmarking;
mod cli;
mod command;
mod rpc;

use polkadot_sdk::*;

fn main() -> sc_cli::Result<()> {
	command::run()
}
