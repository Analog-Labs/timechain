use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::Table;
use tc_cli::Tc;
use tc_subxt::MetadataVariant;
use time_primitives::{Network, NetworkId};

#[derive(Clone, Debug)]
pub struct RelGasPrice {
	pub num: u128,
	pub den: u128,
}

impl FromStr for RelGasPrice {
	type Err = anyhow::Error;

	fn from_str(rel_gas_price: &str) -> Result<Self> {
		let (num, den) = rel_gas_price.split_once('/').context(
			"invalid relative gas price, expected a ratio of unsigned integers separated by '/'",
		)?;
		Ok(Self {
			num: num.parse()?,
			den: den.parse()?,
		})
	}
}

#[derive(Parser, Debug)]
struct Args {
	#[arg(long, default_value = "/etc/config.yaml")]
	config: PathBuf,
	#[arg(long)]
	timechain_metadata: Option<MetadataVariant>,
	#[arg(long, default_value = "/etc/timechain_admin")]
	timechain_keyfile: PathBuf,
	#[arg(long, default_value = "ws://validator:9944")]
	timechain_url: String,
	#[arg(long, default_value = "/etc/target_admin")]
	target_keyfile: PathBuf,
	#[clap(subcommand)]
	cmd: Command,
}

impl Args {
	async fn tc(&self) -> Result<Tc> {
		let tc = Tc::new(
			&self.config,
			&self.timechain_keyfile,
			self.timechain_metadata.as_ref(),
			&self.timechain_url,
		)
		.await?;
		Ok(tc)
	}
}

#[derive(Parser, Debug)]
enum Command {
	Networks {
		#[clap(subcommand)]
		cmd: Option<NetworksCommand>,
	},
	Gateway {
		network: NetworkId,
		#[clap(subcommand)]
		cmd: Option<GatewayCommand>,
	},
	Wallet {
		network: NetworkId,
		#[clap(subcommand)]
		cmd: Option<WalletCommand>,
	},
}

#[derive(Parser, Debug)]
enum NetworksCommand {
	Add { chain_name: String, chain_network: String },
}

#[derive(Parser, Debug)]
enum GatewayCommand {
	Deploy,
	SetAdmin {
		admin: String,
	},
	SetRouting {
		network: NetworkId,
		gateway: Option<String>,
		rel_gas_price: Option<RelGasPrice>,
		gas_limit: Option<u64>,
		base_fee: Option<u128>,
	},
}

#[derive(Parser, Debug)]
enum WalletCommand {
	Faucet,
	Transfer { address: String, amount: String },
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();
	let args = Args::parse();
	let tc = args.tc().await?;
	match args.cmd {
		Command::Networks { cmd } => {
			let Some(cmd) = cmd else {
				let table = tc.networks().await?;
				let table = Table::new(table);
				println!("{table}");
				return Ok(());
			};
			match cmd {
				NetworksCommand::Add { chain_name, chain_network } => {
					tc.add_network(chain_name, chain_network).await?;
				},
			}
		},
		Command::Gateway { network, cmd } => {
			let Some(cmd) = cmd else {
				let table = tc.gateway_routing(network).await?;
				let table = Table::new(table);
				println!("{table}");
				return Ok(());
			};
			match cmd {
				GatewayCommand::Deploy => {
					tc.deploy_gateway(network).await?;
				},
				GatewayCommand::SetAdmin { admin } => {
					tc.set_gateway_admin(network, admin).await?;
				},
				GatewayCommand::SetRouting {
					network: network_id,
					gateway,
					rel_gas_price,
					gas_limit,
					base_fee,
				} => {
					let gateway = if let Some(gateway) = gateway {
						tc.parse_address(network, &gateway)?.into()
					} else {
						Default::default()
					};
					tc.set_gateway_routing(
						network,
						Network {
							network_id,
							gateway,
							relative_gas_price: rel_gas_price
								.map(|r| (r.num, r.den))
								.unwrap_or_default(),
							gas_limit: gas_limit.unwrap_or_default(),
							base_fee: base_fee.unwrap_or_default(),
						},
					)
					.await?;
				},
			}
		},
		Command::Wallet { network, cmd } => {
			let Some(cmd) = cmd else {
				let entry = tc.wallet(network).await?;
				let table = Table::new(vec![entry]);
				println!("{table}");
				return Ok(());
			};
			match cmd {
				WalletCommand::Faucet => {
					tc.faucet(network).await?;
				},
				WalletCommand::Transfer { address, amount } => {
					tc.transfer(network, &address, &amount).await?;
				},
			}
		},
	}
	Ok(())
}
