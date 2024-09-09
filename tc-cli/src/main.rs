use anyhow::{Context, Result};
use clap::Parser;
use gmp_evm::Address;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::Table;
use tc_cli::Tc;
use tc_subxt::MetadataVariant;
use time_primitives::{Network, NetworkId};

#[derive(Clone, Debug)]
pub struct ChainNetwork {
	pub id: NetworkId,
	pub url: String,
}

impl FromStr for ChainNetwork {
	type Err = anyhow::Error;

	fn from_str(network: &str) -> Result<Self> {
		let (id, url) = network.split_once(';').context(
			"invalid network, expected network id followed by a semicolon followed by the rpc url",
		)?;
		Ok(Self {
			id: id.parse()?,
			url: url.into(),
		})
	}
}

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
	#[arg(long, default_values = ["3;ws://ethereum:8545", "6;ws://astar:9944"])]
	network: Vec<ChainNetwork>,
	#[arg(long)]
	timechain_metadata: Option<MetadataVariant>,
	#[arg(long, default_value = "/etc/alice")]
	timechain_keyfile: PathBuf,
	#[arg(long, default_value = "ws://validator:9944")]
	timechain_url: String,
	#[arg(long, default_value = "/etc/keyfile")]
	target_keyfile: PathBuf,
	#[arg(long, default_value = "/etc/contracts/GatewayProxy.sol/GatewayProxy.json")]
	proxy_gateway_contract: PathBuf,
	#[arg(long, default_value = "/etc/contracts/Gateway.sol/Gateway.json")]
	gateway_contract: PathBuf,
	#[arg(long, default_value = "/etc/contracts/test_contract.sol/VotingContract.json")]
	tester_contract: PathBuf,
	#[clap(subcommand)]
	cmd: Command,
}

impl Args {
	async fn tc(&self) -> Result<Tc> {
		let mut tc =
			Tc::new(&self.timechain_keyfile, self.timechain_metadata.as_ref(), &self.timechain_url)
				.await?;
		for network in &self.network {
			tc.add_connector(
				network.id,
				&network.url,
				&self.target_keyfile,
				&self.gateway_contract,
				&self.proxy_gateway_contract,
				&self.tester_contract,
			)
			.await?;
		}
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
		gateway: Option<Address>,
		rel_gas_price: Option<RelGasPrice>,
		gas_limit: Option<u64>,
		base_fee: Option<u128>,
	},
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
					tc.set_gateway_routing(
						network,
						Network {
							network_id,
							gateway: gateway.map(Into::into).unwrap_or_default(),
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
	}
	Ok(())
}
