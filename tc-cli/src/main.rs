use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tabled::{Table, Tabled};
use tc_cli::{Batch, Chronicle, Member, Message, Network, Shard, Task, Tc};
use time_primitives::{
	traits::IdentifyAccount, BatchId, GatewayOp, GmpEvent, GmpMessage, Network as Route, NetworkId,
	ShardId, TaskId,
};

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
	#[clap(subcommand)]
	cmd: Command,
}

impl Args {
	async fn tc(&self) -> Result<Tc> {
		let tc = Tc::new(&self.config).await?;
		Ok(tc)
	}
}

#[derive(Parser, Debug)]
enum Command {
	// balances
	Address {
		#[arg(long)]
		network: Option<NetworkId>,
	},
	Faucet {
		network: NetworkId,
	},
	Balance {
		#[arg(long)]
		network: Option<NetworkId>,
		#[arg(long)]
		address: Option<String>,
	},
	Transfer {
		#[arg(long)]
		network: Option<NetworkId>,
		address: String,
		amount: String,
	},
	// read data
	FetchTokenPriceData,
	Networks,
	Chronicles,
	Shards,
	Members {
		shard: ShardId,
	},
	Routes {
		network: NetworkId,
	},
	Events {
		network: NetworkId,
		start: u64,
		end: u64,
	},
	Messages {
		network: NetworkId,
		tester: String,
		start: u64,
		end: u64,
	},
	Task {
		task: TaskId,
	},
	Batch {
		batch: BatchId,
	},
	Message {
		message: String,
	},
	// management
	Deploy,
	RegisterShards {
		network: NetworkId,
	},
	SetGatewayAdmin {
		network: NetworkId,
		admin: String,
	},
	RedeployGateway {
		network: NetworkId,
	},
	DeployTester {
		network: NetworkId,
	},
	SendMessage {
		network: NetworkId,
		tester: String,
		dest: NetworkId,
		dest_addr: String,
	},
	SmokeTest {
		src: NetworkId,
		dest: NetworkId,
	},
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
	read_events: TaskId,
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
			read_events: self.read_events,
		})
	}
}

#[derive(Tabled)]
struct ChronicleEntry {
	network: NetworkId,
	account: String,
	peer_id: String,
	status: String,
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
			status: self.status.to_string(),
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
			key: self.key.map(hex::encode).unwrap_or_default(),
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
	gas_limit: u64,
	base_fee: u128,
}

impl IntoRow for Route {
	type Row = RouteEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(RouteEntry {
			network: self.network_id,
			gateway: tc.format_address(Some(self.network_id), self.gateway)?,
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
	id: String,
	source_network: NetworkId,
	source_address: String,
	dest_network: NetworkId,
	dest_address: String,
}

impl IntoRow for GmpMessage {
	type Row = MessageEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(MessageEntry {
			id: self.to_string(),
			source_network: self.src_network,
			source_address: tc.format_address(Some(self.src_network), self.src)?,
			dest_network: self.dest_network,
			dest_address: tc.format_address(Some(self.dest_network), self.dest)?,
		})
	}
}

#[derive(Tabled)]
struct TaskEntry {
	task: TaskId,
	descriptor: String,
	output: String,
	shard: String,
	submitter: String,
}

impl IntoRow for Task {
	type Row = TaskEntry;

	fn into_row(self, tc: &Tc) -> Result<Self::Row> {
		Ok(TaskEntry {
			task: self.task,
			descriptor: self.descriptor.to_string(),
			output: match self.output {
				Some(Ok(())) => "complete".to_string(),
				Some(Err(err)) => err,
				None => "in progress".to_string(),
			},
			shard: match self.shard {
				Some(shard) => shard.to_string(),
				None => "unassigned".to_string(),
			},
			submitter: match self.submitter {
				Some(submitter) => tc.format_address(None, submitter.into_account().into())?,
				None => "".to_string(),
			},
		})
	}
}

#[derive(Tabled)]
struct BatchEntry {
	batch: BatchId,
	sign: TaskId,
	sig: String,
	submit: String,
}

impl IntoRow for Batch {
	type Row = BatchEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchEntry {
			batch: self.batch,
			sign: self.sign,
			sig: self.sig.map(hex::encode).unwrap_or_default(),
			submit: self.submit.map(|t| t.to_string()).unwrap_or_default(),
		})
	}
}

#[derive(Tabled)]
struct BatchOpEntry {
	op: String,
}

impl IntoRow for GatewayOp {
	type Row = BatchOpEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(BatchOpEntry { op: self.to_string() })
	}
}

#[derive(Tabled)]
struct MessageInfoEntry {
	message: String,
	recv: String,
	batch: String,
	exec: String,
}

impl IntoRow for Message {
	type Row = MessageInfoEntry;

	fn into_row(self, _tc: &Tc) -> Result<Self::Row> {
		Ok(MessageInfoEntry {
			message: hex::encode(self.message),
			recv: self.recv.map(|t| t.to_string()).unwrap_or_default(),
			batch: self.batch.map(|b| b.to_string()).unwrap_or_default(),
			exec: self.exec.map(|t| t.to_string()).unwrap_or_default(),
		})
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
		Command::Address { network } => {
			let address = tc.address(network)?;
			let address = tc.format_address(network, address)?;
			println!("{address}");
		},
		Command::Balance { network, address } => {
			let address = if let Some(address) = address {
				tc.parse_address(network, &address)?
			} else {
				tc.address(network)?
			};
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
		Command::FetchTokenPriceData => {
			tc.fetch_token_prices().await;
		},
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
		Command::Task { task } => {
			let task = tc.task(task).await?;
			print_table(&tc, vec![task])?;
		},
		Command::Batch { batch } => {
			let mut batch = tc.batch(batch).await?;
			let ops = std::mem::take(&mut batch.msg.ops);
			print_table(&tc, vec![batch])?;
			print_table(&tc, ops)?;
		},
		Command::Message { message } => {
			let message = hex::decode(message)?
				.try_into()
				.map_err(|_| anyhow::anyhow!("invalid message id"))?;
			let message = tc.message(message).await?;
			print_table(&tc, vec![message])?;
		},
		// management
		Command::Deploy => {
			tc.deploy().await?;
		},
		Command::RegisterShards { network } => {
			tc.register_shards(network).await?;
		},
		Command::SetGatewayAdmin { network, admin } => {
			let admin = tc.parse_address(Some(network), &admin)?;
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
		Command::SendMessage {
			network,
			tester,
			dest,
			dest_addr,
		} => {
			let tester = tc.parse_address(Some(network), &tester)?;
			let dest_addr = tc.parse_address(Some(dest), &dest_addr)?;
			let msg_id = tc.send_message(network, tester, dest, dest_addr).await?;
			println!("{}", hex::encode(msg_id));
		},
		Command::SmokeTest { src, dest } => {
			tc.deploy().await?;
			let (src_addr, _src_block) = tc.deploy_tester(src).await?;
			let (dest_addr, dest_block) = tc.deploy_tester(dest).await?;
			let msg_id = tc.send_message(src, src_addr, dest, dest_addr).await?;
			tracing::info!(
				"sent message to {} {}",
				dest,
				tc.format_address(Some(dest), dest_addr)?
			);
			while tc.find_online_shard_keys(src).await?.is_empty() {
				tracing::info!("waiting for shards to come online");
				let shards = tc.shards().await?;
				print_table(&tc, shards)?;
				tokio::time::sleep(Duration::from_secs(1)).await;
			}
			while tc.find_online_shard_keys(dest).await?.is_empty() {
				tracing::info!("waiting for shards to come online");
				let shards = tc.shards().await?;
				print_table(&tc, shards)?;
				tokio::time::sleep(Duration::from_secs(1)).await;
			}
			tc.register_shards(src).await?;
			tc.register_shards(dest).await?;
			let msg = loop {
				let msgs = tc.messages(dest, dest_addr, dest_block..(dest_block + 1000)).await?;
				if let Some(msg) = msgs.into_iter().find(|msg| msg.message_id() == msg_id) {
					break msg;
				}
				tracing::info!("waiting for message {}", hex::encode(msg_id));
				let message = tc.message(msg_id).await?;
				print_table(&tc, vec![message])?;
				tokio::time::sleep(Duration::from_secs(1)).await;
			};
			print_table(&tc, vec![msg])?;
		},
	}
	std::process::exit(0);
}
