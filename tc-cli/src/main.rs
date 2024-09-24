use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::{Table, Tabled};
use tc_cli::{Member, Network, Shard, Tc};
use tc_subxt::MetadataVariant;
use time_primitives::{GmpEvent, GmpMessage, Network as Route, NetworkId};

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
	// balances
	Faucet { network: NetworkId },
	Balance { network: Option<NetworkId>, address: String },
	Transfer { network: Option<NetworkId>, address: String, amount: String },
	// read data
	Networks,
	Members,
	Shards,
	Routes { network: NetworkId },
	Events { network: NetworkId, start: u64, end: u64 },
	Messages { network: NetworkId, tester: String, start: u64, end: u64 },
	// management
	Deploy,
	RegisterShards { network: NetworkId },
	SetGatewayAdmin { network: NetworkId, admin: String },
	RedeployGateway { network: NetworkId },
	DeployTester { network: NetworkId },
	SendMessage { network: NetworkId, tester: String },
}

trait ToRow {
	type Row: Tabled;

	fn to_row(&self, tc: &Tc, network: Option<NetworkId>) -> Result<Self::Row>;
}

fn print_table<R: ToRow>(tc: &Tc, network: Option<NetworkId>, table: Vec<R>) -> Result<()> {
	let mut rows = Vec::with_capacity(table.len());
	for row in table {
		rows.push(row.to_row(tc, network)?);
	}
	println!("{}", Table::new(rows));
	Ok(())
}

#[derive(Tabled)]
struct NetworkEntry {}

impl ToRow for Network {
	type Row = NetworkEntry;

	fn to_row(&self, _tc: &Tc, _network: Option<NetworkId>) -> Result<Self::Row> {
		Ok(NetworkEntry {})
	}
}

#[derive(Tabled)]
struct MemberEntry {}

impl ToRow for Member {
	type Row = MemberEntry;

	fn to_row(&self, _tc: &Tc, _network: Option<NetworkId>) -> Result<Self::Row> {
		Ok(MemberEntry {})
	}
}

#[derive(Tabled)]
struct ShardEntry {}

impl ToRow for Shard {
	type Row = ShardEntry;

	fn to_row(&self, _tc: &Tc, _network: Option<NetworkId>) -> Result<Self::Row> {
		Ok(ShardEntry {})
	}
}

#[derive(Tabled)]
struct RouteEntry {
	network: NetworkId,
	gateway: String,
	relative_gas_price: String,
	gas_limit: u64,
	base_fee: u128,
}

impl ToRow for Route {
	type Row = RouteEntry;

	fn to_row(&self, tc: &Tc, network: Option<NetworkId>) -> Result<Self::Row> {
		let (num, den) = self.relative_gas_price;
		Ok(RouteEntry {
			network: self.network_id,
			gateway: tc.format_address(network, self.gateway)?,
			relative_gas_price: format!("{}", num as f64 / den as f64),
			gas_limit: self.gas_limit,
			base_fee: self.base_fee,
		})
	}
}

#[derive(Tabled)]
struct EventEntry {
	event: String,
}

impl ToRow for GmpEvent {
	type Row = EventEntry;

	fn to_row(&self, _tc: &Tc, _network: Option<NetworkId>) -> Result<Self::Row> {
		Ok(EventEntry { event: self.to_string() })
	}
}

#[derive(Tabled)]
struct MessageEntry {
	message: String,
}

impl ToRow for GmpMessage {
	type Row = MessageEntry;

	fn to_row(&self, _tc: &Tc, _network: Option<NetworkId>) -> Result<Self::Row> {
		Ok(MessageEntry { message: self.to_string() })
	}
}

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();
	let args = Args::parse();
	let tc = args.tc().await?;
	match args.cmd {
		// balances
		Command::Faucet { network } => {
			tc.faucet(network).await?;
		},
		Command::Balance { network, address } => {
			let address = tc.parse_address(network, &address)?;
			let balance = tc.balance(network, address).await?;
			let balance = tc.format_balance(network, balance)?;
			println!("{balance}");
		},
		Command::Transfer { network, address, amount } => {
			let address = tc.parse_address(network, &address)?;
			let amount = tc.parse_balance(network, &amount)?;
			tc.transfer(network, address, amount).await?;
		},
		// read data
		Command::Networks => {
			let networks = tc.networks().await?;
			print_table(&tc, None, networks)?;
		},
		Command::Members => {
			let members = tc.members().await?;
			print_table(&tc, None, members)?;
		},
		Command::Shards => {
			let shards = tc.shards().await?;
			print_table(&tc, None, shards)?;
		},
		Command::Routes { network } => {
			let routes = tc.routes(network).await?;
			print_table(&tc, Some(network), routes)?;
		},
		Command::Events { network, start, end } => {
			let events = tc.events(network, start..end).await?;
			print_table(&tc, Some(network), events)?;
		},
		Command::Messages { network, tester, start, end } => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let msgs = tc.messages(network, tester, start..end).await?;
			print_table(&tc, Some(network), msgs)?;
		},
		// management
		Command::Deploy => {
			tc.deploy().await?;
		},
		Command::RegisterShards { network } => {
			tc.register_shards(network).await?;
		},
		Command::SetGatewayAdmin { network, admin } => {
			tc.set_gateway_admin(network, admin).await?;
		},
		Command::RedeployGateway { network } => {
			tc.redeploy_gateway(network).await?;
		},
		Command::DeployTester { network } => {
			let (address, block) = tc.deploy_tester(network).await?;
			let address = tc.format_address(Some(network), address)?;
			println!("{address} {block}");
		},
		Command::SendMessage { network: _, tester: _ } => {
			//tc.send_message(network, tester).await?;
			todo!()
		},
	}
	Ok(())
}
