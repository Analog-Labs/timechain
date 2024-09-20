use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use redb::{
	Database, Key, MultimapTableDefinition, ReadableTable, TableDefinition, TypeName, Value,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::ops::Range;
use std::path::Path;
use std::pin::Pin;
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;
use time_primitives::{
	Address, BatchId, ConnectorParams, GatewayMessage, GatewayOp, GmpEvent, GmpMessage, GmpParams,
	IChain, IConnector, IConnectorAdmin, Network, NetworkId, TssPublicKey, TssSignature,
};

const BLOCK_TIME: u64 = 1;
const FINALIZATION_TIME: u64 = 2;
const FAUCET: u128 = 1_000_000_000;
const BALANCE: TableDefinition<Address, u128> = TableDefinition::new("balance");
const ADMIN: TableDefinition<Address, Address> = TableDefinition::new("admin");
const EVENTS: MultimapTableDefinition<(Address, u64), Bincode<GmpEvent>> =
	MultimapTableDefinition::new("events");
const SHARDS: MultimapTableDefinition<Address, TssPublicKey> =
	MultimapTableDefinition::new("shards");
const NETWORKS: TableDefinition<(Address, NetworkId), Bincode<Network>> =
	TableDefinition::new("networks");
const GATEWAY: TableDefinition<Address, Address> = TableDefinition::new("gateway");
const TESTERS: MultimapTableDefinition<Address, Address> = MultimapTableDefinition::new("testers");

pub struct Connector {
	network_id: NetworkId,
	address: Address,
	db: Database,
	genesis: SystemTime,
	_tmpfile: Option<NamedTempFile>,
}

fn block(genesis: SystemTime) -> u64 {
	let elapsed = SystemTime::now().duration_since(genesis).unwrap();
	elapsed.as_secs() / BLOCK_TIME
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
		let genesis = self.genesis;
		futures::stream::repeat(0)
			.then(move |_| async move {
				tokio::time::sleep(Duration::from_secs(FINALIZATION_TIME)).await;
				block(genesis)
			})
			.boxed()
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
		gateway: Address,
		batch: BatchId,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		let bytes = msg.encode(batch);
		let hash = GmpParams::new(self.network_id(), gateway).hash(&bytes);
		time_primitives::verify_signature(signer, &hash, sig)
			.map_err(|_| "invalid signature".to_string())?;
		(|| {
			let tx = self.db.begin_write()?;
			let mut events = tx.open_multimap_table(EVENTS)?;
			let mut shards = tx.open_multimap_table(SHARDS)?;
			let block = block(self.genesis);
			for op in &msg.ops {
				match op {
					GatewayOp::RegisterShard(key) => {
						shards.insert(gateway, key)?;
						events.insert((gateway, block), GmpEvent::ShardRegistered(*key))?;
					},
					GatewayOp::UnregisterShard(key) => {
						shards.remove(gateway, key)?;
						events.insert((gateway, block), GmpEvent::ShardUnregistered(*key))?;
					},
					GatewayOp::SendMessage(msg) => {
						events.insert((msg.dest, block), GmpEvent::MessageReceived(msg.clone()))?;
						events.insert(
							(gateway, block),
							GmpEvent::MessageExecuted(msg.message_id()),
						)?;
					},
				}
			}
			events.insert((gateway, block), GmpEvent::BatchExecuted(batch))?;
			Ok(())
		})()
		.map_err(|err: anyhow::Error| err.to_string())
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
		let block = block(self.genesis);
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

	async fn set_shards(&self, gateway: Address, keys: &[TssPublicKey]) -> Result<()> {
		let tx = self.db.begin_write()?;
		let mut events = tx.open_multimap_table(EVENTS)?;
		let mut shards = tx.open_multimap_table(SHARDS)?;
		let block = block(self.genesis);
		let values = shards.remove_all(gateway)?;
		let keys: BTreeSet<_> = keys.into_iter().copied().collect();
		let mut old_keys = BTreeSet::new();
		for value in values {
			let old_key = value?.value();
			old_keys.insert(old_key);
			if !keys.contains(&old_key) {
				events.insert((gateway, block), GmpEvent::ShardUnregistered(old_key))?;
			}
		}
		for key in keys {
			shards.insert(gateway, key)?;
			if !old_keys.contains(&key) {
				events.insert((gateway, block), GmpEvent::ShardRegistered(key))?;
			}
		}
		Ok(())
	}

	async fn networks(&self, gateway: Address) -> Result<Vec<Network>> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(NETWORKS)?;
		let mut networks = vec![];
		for r in t.iter()? {
			let (k, v) = r?;
			let (g, _) = k.value();
			if g != gateway {
				continue;
			}
			let network = v.value();
			networks.push(network);
		}
		Ok(networks)
	}

	async fn set_network(&self, gateway: Address, new_network: Network) -> Result<()> {
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(NETWORKS)?;
		let mut network = t
			.remove((gateway, new_network.network_id))?
			.map(|g| g.value())
			.unwrap_or(new_network.clone());
		if new_network.gateway != [0; 32] {
			network.gateway = new_network.gateway;
		}
		if new_network.relative_gas_price != (0, 0) {
			network.relative_gas_price = new_network.relative_gas_price;
		}
		if new_network.gas_limit != 0 {
			network.gas_limit = new_network.gas_limit;
		}
		if new_network.base_fee != 0 {
			network.base_fee = new_network.base_fee;
		}
		t.insert((gateway, network.network_id), network)?;
		Ok(())
	}

	async fn deploy_test(&self, gateway: Address, _path: &Path) -> Result<(Address, u64)> {
		let mut tester = [0; 32];
		getrandom::getrandom(&mut tester).unwrap();
		let block = block(self.genesis);
		let tx = self.db.begin_write()?;
		let mut t = tx.open_table(GATEWAY)?;
		t.insert(tester, gateway)?;
		let mut t = tx.open_multimap_table(TESTERS)?;
		t.insert(gateway, tester)?;
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

	async fn send_message(&self, addr: Address, msg: GmpMessage) -> Result<()> {
		let tx = self.db.begin_write()?;
		let t = tx.open_table(GATEWAY)?;
		let gateway = t.get(addr)?.context("tester not deployed")?.value();
		let mut t = tx.open_multimap_table(EVENTS)?;
		let block = block(self.genesis);
		t.insert((gateway, block), GmpEvent::MessageReceived(msg))?;
		Ok(())
	}

	async fn recv_messages(&self, addr: Address, blocks: Range<u64>) -> Result<Vec<GmpMessage>> {
		let tx = self.db.begin_read()?;
		let t = tx.open_multimap_table(EVENTS)?;
		let mut msgs = vec![];
		for block in blocks {
			for event in t.get((addr, block))? {
				let event = event?.value();
				let GmpEvent::MessageReceived(msg) = event else {
					continue;
				};
				msgs.push(msg);
			}
		}
		Ok(msgs)
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
