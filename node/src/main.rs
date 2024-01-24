//! Substrate Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
mod chronicle;
#[macro_use]
mod service;
mod benchmarking;
mod cli;
mod command;
mod rpc;

fn main() -> sc_cli::Result<()> {
	command::run()
}
