use crate::{
	benchmarking::{inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder},
	chain_spec,
	cli::{Cli, Subcommand},
	service::{self, FullClient},
};

use polkadot_sdk::*;

use frame_benchmarking_cli::ExtrinsicFactory;
use frame_benchmarking_cli::{BenchmarkCmd, SUBSTRATE_REFERENCE_HARDWARE};
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;
use sp_keyring::Sr25519Keyring;
use sp_runtime::traits::HashingFor;
use time_primitives::Block;

use std::sync::Arc;
use timechain_runtime::{
	ExistentialDeposit, Runtime, RuntimeApi, RUNTIME_VARIANT, VERSION, WASM_BINARY,
};

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		if WASM_BINARY.is_some() {
			format!("Timechain Node ('{}' v{})", RUNTIME_VARIANT, VERSION.spec_version)
		} else {
			"Timechain Node".into()
		}
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"support.analog.one".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			// Pre-release networks
			"mainnet" => Box::new(chain_spec::GenesisKeysConfig::default().to_mainnet()?),
			// Choose latest live network by default
			"testnet" | "" => Box::new(chain_spec::ChainSpec::from_json_bytes(
				&include_bytes!("chains/testnet.raw.json")[..],
			)?),
			// Internal development networks
			"staging" => Box::new(
				chain_spec::GenesisKeysConfig::from_json_bytes(
					&include_bytes!("chains/internal.keys.json")[..],
				)?
				.to_development("staging")?,
			),
			"integration" => Box::new(
				chain_spec::GenesisKeysConfig::from_json_bytes(
					&include_bytes!("chains/internal.keys.json")[..],
				)?
				.to_development("integration")?,
			),
			"development" => Box::new(
				chain_spec::GenesisKeysConfig::from_json_bytes(
					&include_bytes!("chains/internal.keys.json")[..],
				)?
				.to_development("development")?,
			),
			// Local testing networks
			"dev" => Box::new(chain_spec::GenesisKeysConfig::default().to_local()?),
			// External chain spec file
			path => {
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?)
			},
		})
	}
}

/// Parse command line arguments into service configuration.
pub fn run() -> sc_cli::Result<()> {
	// Parse command line arguments
	let cli = Cli::from_args();

	// Parse subcommand to determine what to run
	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full::<RuntimeApi>(config, cli).map_err(sc_cli::Error::Service)
			})
		},
		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| cmd.run::<Block, RuntimeApi>(config))
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							 You can enable it with `--features runtime-benchmarks`."
									.into(),
							);
						}

						cmd.run_with_spec::<HashingFor<Block>, ()>(Some(config.chain_spec))
					},
					BenchmarkCmd::Block(cmd) => {
						// ensure that we keep the task manager alive
						let partial = service::new_partial::<RuntimeApi>(&config)?;
						cmd.run(partial.client)
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err(
						"Storage benchmarking can be enabled with `--features runtime-benchmarks`."
							.into(),
					),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(cmd) => {
						// ensure that we keep the task manager alive
						let partial = service::new_partial::<RuntimeApi>(&config)?;
						let db = partial.backend.expose_db();
						let storage = partial.backend.expose_storage();

						cmd.run(config, partial.client, db, storage)
					},
					BenchmarkCmd::Overhead(cmd) => {
						// ensure that we keep the task manager alive
						let partial = service::new_partial::<RuntimeApi>(&config)?;
						let ext_builder =
							RemarkBuilder::<Runtime, RuntimeApi>::new(partial.client.clone());

						cmd.run(
							config,
							partial.client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_builder,
						)
					},
					BenchmarkCmd::Extrinsic(cmd) => {
						// ensure that we keep the task manager alive
						let partial = service::new_partial::<RuntimeApi>(&config)?;
						// Register the *Remark* and *TKA* builders.
						let ext_factory = ExtrinsicFactory(vec![
							Box::new(RemarkBuilder::<Runtime, RuntimeApi>::new(
								partial.client.clone(),
							)),
							Box::new(TransferKeepAliveBuilder::<Runtime, RuntimeApi>::new(
								partial.client.clone(),
								Sr25519Keyring::Alice.to_account_id(),
								ExistentialDeposit::get(),
							)),
						]);

						cmd.run(
							partial.client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_factory,
						)
					},
					BenchmarkCmd::Machine(cmd) => {
						cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
					},
				}
			})
		},
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = service::new_partial::<RuntimeApi>(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial::<RuntimeApi>(&config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial::<RuntimeApi>(&config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = service::new_partial::<RuntimeApi>(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client, task_manager, backend, ..
				} = service::new_partial::<RuntimeApi>(&config)?;
				let aux_revert =
					Box::new(|client: Arc<FullClient<RuntimeApi>>, backend, blocks| {
						sc_consensus_babe::revert(client.clone(), backend, blocks)?;
						sc_consensus_grandpa::revert(client, blocks)?;
						Ok(())
					});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
	}
}
