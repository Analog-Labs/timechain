use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use tabled::{Table, Tabled};
use tc_cli::{Chronicle, Member, Network, Shard, Tc};
use tc_subxt::MetadataVariant;
use time_primitives::{GmpEvent, GmpMessage, Network as Route, NetworkId, ShardId};

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
	Chronicles,
	Shards,
	Members { shard: ShardId },
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

trait IntoRow {
	type Row: Tabled;

	fn into_row(self, tc: &Tc) -> Result<Self::Row>;
}

fn print_table<R: IntoRow>(tc: &Tc, table: Vec<R>) -> Result<()> {
	let mut rows = Vec::with_capacity(table.len());
	for row in table {
		rows.push(row.into_row(tc)?);
	}
	println!("{}", Table::new(rows));
	Ok(())
}

#[derive(Tabled)]
struct NetworkEntry {
	network: NetworkId,
	chain_name: String,
	chain_network: String,
	gateway: String,
	gateway_balance: String,
	admin: String,
	admin_balance: String,
}

impl IntoRow for Network {
	type Row = NetworkEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(NetworkEntry {
			network: self.network,
			chain_name: self.chain_name,
			chain_network: self.chain_network,
			gateway: tc.format_address(Some(self.network), self.gateway)?,
			gateway_balance: tc.format_balance(Some(self.network), self.gateway_balance)?,
			admin: tc.format_address(Some(self.network), self.admin)?,
			admin_balance: tc.format_balance(Some(self.network), self.admin_balance)?,
		})
	}
}

#[derive(Tabled)]
struct ChronicleEntry {
	network: NetworkId,
	account: String,
	peer_id: String,
	//status: String,
	balance: String,
	target_address: String,
	target_balance: String,
}

impl IntoRow for Chronicle {
	type Row = ChronicleEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(ChronicleEntry {
			network: self.network,
			account: tc.format_address(None, self.account.into())?,
			peer_id: self.peer_id,
			balance: tc.format_balance(None, self.balance)?,
			target_address: tc.format_address(Some(self.network), self.target_address)?,
			target_balance: tc.format_balance(Some(self.network), self.target_balance)?,
		})
	}
}

#[derive(Tabled)]
struct ShardEntry {
	shard: ShardId,
	network: NetworkId,
	status: String,
	key: String,
	registered: String,
	size: u16,
	threshold: u16,
}

impl IntoRow for Shard {
	type Row = ShardEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(ShardEntry {
			shard: self.shard,
			network: self.network,
			status: self.status.to_string(),
			key: self.key.map(|key| hex::encode(&key)).unwrap_or_default(),
			registered: self.registered.to_string(),
			size: self.size,
			threshold: self.threshold,
		})
	}
}

#[derive(Tabled)]
struct MemberEntry {
	account: String,
	status: String,
}

impl IntoRow for Member {
	type Row = MemberEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(MemberEntry {
			account: self.account.to_string(),
			status: self.status.to_string(),
		})
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

impl IntoRow for Route {
	type Row = RouteEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		let (num, den) = self.relative_gas_price;
		Ok(RouteEntry {
			network: self.network_id,
			gateway: tc.format_address(Some(self.network_id), self.gateway)?,
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

impl IntoRow for GmpEvent {
	type Row = EventEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(EventEntry { event: self.to_string() })
	}
}

#[derive(Tabled)]
struct MessageEntry {
	message: String,
}

impl IntoRow for GmpMessage {
	type Row = MessageEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
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
			print_table(&tc, networks)?;
		},
		Command::Chronicles => {
			let chronicles = tc.chronicles().await?;
			print_table(&tc, chronicles)?;
		},
		Command::Shards => {
			let shards = tc.shards().await?;
			print_table(&tc, shards)?;
		},
		Command::Members { shard } => {
			let members = tc.members(shard).await?;
			print_table(&tc, members)?;
		},
		Command::Routes { network } => {
			let routes = tc.routes(network).await?;
			print_table(&tc, routes)?;
		},
		Command::Events { network, start, end } => {
			let events = tc.events(network, start..end).await?;
			print_table(&tc, events)?;
		},
		Command::Messages { network, tester, start, end } => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let msgs = tc.messages(network, tester, start..end).await?;
			print_table(&tc, msgs)?;
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
			// TODO: tc.send_message(network, tester).await?;
			todo!()
		},
	}
	Ok(())
}
