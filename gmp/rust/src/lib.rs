use anyhow::{Context, Result};
use futures::Stream;
use redb::{
	Database, Key, MultimapTableDefinition, ReadableTable, TableDefinition, TypeName, Value,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Range;
use std::path::Path;
use std::pin::Pin;
use std::time::SystemTime;
use tempfile::NamedTempFile;
use time_primitives::{
	Address, ConnectorParams, GmpEvent, GmpMessage, IChain, IConnector, IConnectorAdmin, Network,
	NetworkId, TssPublicKey, TssSignature,
};

const BLOCK_TIME: u64 = 6;
const FAUCET: u128 = 1_000_000_000;
const BALANCE: TableDefinition<Address, u128> = TableDefinition::new("balance");
const ADMIN: TableDefinition<Address, Address> = TableDefinition::new("admin");
const EVENTS: MultimapTableDefinition<(Address, u64), Bincode<GmpEvent>> =
	MultimapTableDefinition::new("events");
const SHARDS: MultimapTableDefinition<Address, TssPublicKey> =
	MultimapTableDefinition::new("shards");
const NETWORKS: MultimapTableDefinition<Address, Bincode<Network>> =
	MultimapTableDefinition::new("networks");
const TESTERS: TableDefinition<Address, Address> = TableDefinition::new("testers");

pub struct Connector {
	network_id: NetworkId,
	address: Address,
	db: Database,
	genesis: SystemTime,
	_tmpfile: Option<NamedTempFile>,
}

impl Connector {
	fn block(&self) -> u64 {
		let elapsed = SystemTime::now().duration_since(self.genesis).unwrap();
		elapsed.as_secs() / BLOCK_TIME
	}
}

fn read_balance<T: ReadableTable<Address, u128>>(table: &T, addr: Address) -> Result<u128> {
	Ok(if let Some(value) = table.get(addr)? { value.value() } else { 0 })
}

fn read_admin<T: ReadableTable<Address, Address>>(table: &T, gateway: Address) -> Result<Address> {
	Ok(table.get(gateway)?.context("invalid gateway")?.value())
}

#[async_trait::async_trait]
impl IChain for Connector {
	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String {
		hex::encode(&address)
	}

	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address> {
		let addr = hex::decode(address).map_err(|_| anyhow::anyhow!("invalid address"))?;
		let addr = addr.try_into().map_err(|_| anyhow::anyhow!("invalid address"))?;
		Ok(addr)
	}

	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		self.network_id
	}

	/// Human readable connector account identifier.
	fn address(&self) -> Address {
		self.address
	}

	async fn faucet(&self) -> Result<()> {
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(BALANCE)?;
		let balance = read_balance(&t, self.address)?;
		t.insert(self.address, balance + FAUCET)?;
		Ok(())
	}

	/// Queries the account balance.
	async fn balance(&self, addr: Address) -> Result<u128> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(BALANCE)?;
		let Some(balance) = t.get(addr)? else {
			return Ok(0);
		};
		Ok(balance.value())
	}

	async fn transfer(&self, address: Address, amount: u128) -> Result<()> {
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(BALANCE)?;
		let balance = read_balance(&t, self.address)?;
		if balance < amount {
			anyhow::bail!("insufficient balance");
		}
		let dest_balance = read_balance(&t, address)?;
		t.insert(self.address, balance - amount)?;
		t.insert(address, dest_balance + amount)?;
		Ok(())
	}

	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send + '_>> {
		todo!()
	}
}

#[async_trait::async_trait]
impl IConnector for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		let mnemonic = std::fs::read_to_string(&params.keyfile)?;
		let address = Address::from(*blake3::hash(mnemonic.as_bytes()).as_bytes());
		let (tmpfile, path) = if params.network == "tempfile" {
			let file = NamedTempFile::new()?;
			let path = file.path().to_owned();
			(Some(file), path)
		} else {
			(None, Path::new(&params.network).to_owned())
		};
		let db = Database::create(&path)?;
		let genesis = std::fs::metadata(&path)?.created()?;
		Ok(Self {
			network_id: params.network_id,
			address,
			db,
			genesis,
			_tmpfile: tmpfile,
		})
	}

	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Address, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
		let tx = self.db.begin_read()?;
		let t = tx.open_multimap_table(EVENTS)?;
		let mut events = vec![];
		for block in blocks {
			let values = t.get((gateway, block))?;
			for value in values {
				let event = value?.value();
				events.push(event);
			}
		}
		Ok(events)
	}

	/// Submits a gmp message to the target chain.
	async fn submit_commands(
		&self,
		_gateway: Address,
		_msg: Vec<u8>,
		_signer: TssPublicKey,
		_sig: TssSignature,
	) -> Result<(), String> {
		todo!()
	}
}

#[async_trait::async_trait]
impl IConnectorAdmin for Connector {
	async fn deploy_gateway(
		&self,
		_gateway: &Path,
		_gateway_impl: &Path,
	) -> Result<(Address, u64)> {
		let mut gateway = [0; 32];
		getrandom::getrandom(&mut gateway).unwrap();
		let block = self.block();
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(ADMIN)?;
		t.insert(gateway, self.address)?;
		Ok((gateway.into(), block))
	}

	async fn redeploy_gateway(&self, gateway: Address, _gateway_impl: &Path) -> Result<()> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(ADMIN)?;
		let admin = read_admin(&t, gateway)?;
		if admin != self.address {
			anyhow::bail!("not admin");
		}
		Ok(())
	}

	async fn admin(&self, gateway: Address) -> Result<Address> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(ADMIN)?;
		let admin = read_admin(&t, gateway)?;
		Ok(admin.into())
	}

	async fn set_admin(&self, gateway: Address, new_admin: Address) -> Result<()> {
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(ADMIN)?;
		let admin = read_admin(&t, gateway)?;
		if admin != self.address {
			anyhow::bail!("not admin");
		}
		t.insert(gateway, new_admin)?;
		Ok(())
	}

	async fn shards(&self, gateway: Address) -> Result<Vec<TssPublicKey>> {
		let tx = self.db.begin_read()?;
		let t = tx.open_multimap_table(SHARDS)?;
		let values = t.get(gateway)?;
		let mut shards = Vec::with_capacity(values.len() as _);
		for value in values {
			let shard = value?.value();
			shards.push(shard);
		}
		Ok(shards)
	}

	async fn set_shards(&self, _gateway: Address, _keys: &[TssPublicKey]) -> Result<()> {
		todo!()
	}

	async fn networks(&self, gateway: Address) -> Result<Vec<Network>> {
		let tx = self.db.begin_read()?;
		let t = tx.open_multimap_table(NETWORKS)?;
		let values = t.get(gateway)?;
		let mut networks = Vec::with_capacity(values.len() as _);
		for value in values {
			let network = value?.value();
			networks.push(network);
		}
		Ok(networks)
	}

	async fn set_network(&self, _gateway: Address, _network: Network) -> Result<()> {
		todo!()
	}

	async fn deploy_test(&self, gateway: Address, _path: &Path) -> Result<(Address, u64)> {
		let mut tester = [0; 32];
		getrandom::getrandom(&mut tester).unwrap();
		let block = self.block();
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(TESTERS)?;
		t.insert(tester, gateway)?;
		Ok((tester, block))
	}

	async fn estimate_message_cost(
		&self,
		_gateway: Address,
		_dest: NetworkId,
		msg_size: usize,
	) -> Result<u128> {
		Ok(msg_size as u128 * 100)
	}

	async fn send_message(&self, _addr: Address, _msg: GmpMessage) -> Result<()> {
		todo!()
	}

	async fn recv_messages(&self, _addr: Address, _blocks: Range<u64>) -> Result<Vec<GmpMessage>> {
		todo!()
	}
}

#[derive(Debug)]
pub struct Bincode<T>(pub T);

impl<T> Value for Bincode<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	type SelfType<'a> = T
    where
        Self: 'a;

	type AsBytes<'a> = Vec<u8>
    where
        Self: 'a;

	fn fixed_width() -> Option<usize> {
		None
	}

	fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
	where
		Self: 'a,
	{
		bincode::deserialize(data).unwrap()
	}

	fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
	where
		Self: 'a,
		Self: 'b,
	{
		bincode::serialize(value).unwrap()
	}

	fn type_name() -> TypeName {
		TypeName::new(&format!("Bincode<{}>", std::any::type_name::<T>()))
	}
}

impl<T> Key for Bincode<T>
where
	T: Debug + Serialize + DeserializeOwned + Ord,
{
	fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
		Self::from_bytes(data1).cmp(&Self::from_bytes(data2))
	}
}
