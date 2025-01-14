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
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;
use time_primitives::{
	Address, BatchId, ConnectorParams, GatewayMessage, GatewayOp, GmpEvent, GmpMessage, GmpParams,
	IChain, IConnector, IConnectorAdmin, IConnectorBuilder, MessageId, NetworkId, Route,
	TssPublicKey, TssSignature,
};

const BLOCK_TIME: u64 = 1;
const FINALIZATION_TIME: u64 = 2;
const FAUCET: u128 = 1_000_000_000;

const BLOCKS: TableDefinition<u64, u64> = TableDefinition::new("blocks");
const BALANCE: TableDefinition<Address, u128> = TableDefinition::new("balance");
const ADMIN: TableDefinition<Address, Address> = TableDefinition::new("admin");
const NONCE: TableDefinition<(Address, Address), u64> = TableDefinition::new("nonce");
const EVENTS: MultimapTableDefinition<(Address, u64), Bincode<GmpEvent>> =
	MultimapTableDefinition::new("events");
const SHARDS: MultimapTableDefinition<Address, TssPublicKey> =
	MultimapTableDefinition::new("shards");
const ROUTES: TableDefinition<(Address, NetworkId), Bincode<Route>> =
	TableDefinition::new("routes");
const GATEWAY: TableDefinition<Address, Address> = TableDefinition::new("gateway");
const TESTERS: MultimapTableDefinition<Address, Address> = MultimapTableDefinition::new("testers");

#[derive(Clone)]
pub struct Connector {
	network_id: NetworkId,
	address: Address,
	db: Arc<Database>,
	genesis: SystemTime,
	_tmpfile: Option<Arc<NamedTempFile>>,
}

impl Connector {
	pub fn with_mnemonic(&self, mnemonic: String) -> Self {
		self.with_address(mnemonic_to_address(mnemonic))
	}

	pub fn with_address(&self, address: Address) -> Self {
		let mut clone = Clone::clone(self);
		clone.address = address;
		clone
	}
}

pub fn mnemonic_to_address(mnemonic: String) -> Address {
	*blake3::hash(mnemonic.as_bytes()).as_bytes()
}

pub fn format_address(address: Address) -> String {
	hex::encode(address)
}

pub fn parse_address(address: &str) -> Result<Address> {
	let addr = hex::decode(address).map_err(|_| anyhow::anyhow!("invalid address"))?;
	let addr = addr.try_into().map_err(|_| anyhow::anyhow!("invalid address"))?;
	Ok(addr)
}

pub fn currency() -> (u32, &'static str) {
	(3, "TT")
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
impl IConnectorBuilder for Connector {
	/// Creates a new connector.
	async fn new(params: ConnectorParams) -> Result<Self>
	where
		Self: Sized,
	{
		if params.blockchain != "rust" {
			anyhow::bail!("unsupported blockchain");
		}
		let address = mnemonic_to_address(params.mnemonic);
		let (tmpfile, path) = if params.url == "tempfile" {
			let file = NamedTempFile::new()?;
			let path = file.path().to_owned();
			(Some(Arc::new(file)), path)
		} else {
			(None, Path::new(&params.url).to_owned())
		};
		let db = Database::create(path)?;
		let tx = db.begin_write()?;
		let genesis = {
			let mut blocks = tx.open_table(BLOCKS)?;
			let timestamp = blocks.get(0)?.map(|t| t.value());
			if let Some(timestamp) = timestamp {
				SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp)
			} else {
				let genesis = SystemTime::now();
				blocks.insert(0, genesis.duration_since(SystemTime::UNIX_EPOCH)?.as_secs())?;
				genesis
			}
		};
		tx.open_table(BALANCE)?;
		tx.open_table(ADMIN)?;
		tx.open_table(ROUTES)?;
		tx.open_table(GATEWAY)?;
		tx.open_table(NONCE)?;
		tx.open_multimap_table(EVENTS)?;
		tx.open_multimap_table(SHARDS)?;
		tx.open_multimap_table(TESTERS)?;
		tx.commit()?;
		Ok(Self {
			network_id: params.network_id,
			address,
			db: Arc::new(db),
			genesis,
			_tmpfile: tmpfile,
		})
	}
}

#[async_trait::async_trait]
impl IChain for Connector {
	/// Formats an address into a string.
	fn format_address(&self, address: Address) -> String {
		format_address(address)
	}

	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address> {
		parse_address(address)
	}

	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		self.network_id
	}

	/// Human readable connector account identifier.
	fn address(&self) -> Address {
		self.address
	}

	fn currency(&self) -> (u32, &str) {
		currency()
	}

	async fn faucet(&self) -> Result<()> {
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(BALANCE)?;
			let balance = read_balance(&t, self.address)?;
			t.insert(self.address, balance + FAUCET)?;
		}
		tx.commit()?;
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
		{
			let mut t = tx.open_table(BALANCE)?;
			let balance = read_balance(&t, self.address)?;
			if balance < amount {
				anyhow::bail!("insufficient balance");
			}
			let dest_balance = read_balance(&t, address)?;
			t.insert(self.address, balance - amount)?;
			t.insert(address, dest_balance + amount)?;
		}
		tx.commit()?;
		Ok(())
	}

	async fn finalized_block(&self) -> Result<u64> {
		Ok(block(self.genesis))
	}

	/// Stream of finalized block indexes.
	fn block_stream(&self) -> Pin<Box<dyn Stream<Item = u64> + Send>> {
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
		let hash = GmpParams::new(self.network_id(), gateway).hash(&msg.encode(batch));

		time_primitives::verify_signature(signer, &hash, sig)
			.map_err(|_| "invalid signature".to_string())?;
		(|| {
			let tx = self.db.begin_write()?;
			{
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
							events.insert(
								(msg.dest, block),
								GmpEvent::MessageReceived(msg.clone()),
							)?;
							events.insert(
								(gateway, block),
								GmpEvent::MessageExecuted(msg.message_id()),
							)?;
						},
					}
				}
				events.insert((gateway, block), GmpEvent::BatchExecuted(batch, None))?;
			}
			tx.commit()?;
			Ok(())
		})()
		.map_err(|err: anyhow::Error| err.to_string())
	}
}

#[async_trait::async_trait]
impl IConnectorAdmin for Connector {
	async fn deploy_gateway(
		&self,
		_additional_params: &[u8],
		_gateway: &[u8],
		_gateway_impl: &[u8],
	) -> Result<(Address, u64)> {
		let mut gateway = [0; 32];
		getrandom::getrandom(&mut gateway).unwrap();
		let block = block(self.genesis);
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(ADMIN)?;
			t.insert(gateway, self.address)?;
		}
		tx.commit()?;
		Ok((gateway, block))
	}

	async fn redeploy_gateway(
		&self,
		_additional_params: &[u8],
		gateway: Address,
		_gateway_impl: &[u8],
	) -> Result<()> {
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
		Ok(admin)
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
		{
			let mut events = tx.open_multimap_table(EVENTS)?;
			let mut shards = tx.open_multimap_table(SHARDS)?;
			let block = block(self.genesis);
			let values = shards.remove_all(gateway)?;
			let keys: BTreeSet<_> = keys.iter().copied().collect();
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
		}
		tx.commit()?;
		Ok(())
	}

	async fn routes(&self, gateway: Address) -> Result<Vec<Route>> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(ROUTES)?;
		let mut routes = vec![];
		for r in t.iter()? {
			let (k, v) = r?;
			let (g, _) = k.value();
			if g != gateway {
				continue;
			}
			routes.push(v.value());
		}
		Ok(routes)
	}

	async fn set_route(&self, gateway: Address, new_route: Route) -> Result<()> {
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(ROUTES)?;
			let mut route = t
				.remove((gateway, new_route.network_id))?
				.map(|g| g.value())
				.unwrap_or(new_route.clone());
			if new_route.gateway != [0; 32] {
				route.gateway = new_route.gateway;
			}
			if new_route.relative_gas_price != (0, 0) {
				route.relative_gas_price = new_route.relative_gas_price;
			}
			if new_route.gas_limit != 0 {
				route.gas_limit = new_route.gas_limit;
			}
			if new_route.base_fee != 0 {
				route.base_fee = new_route.base_fee;
			}
			t.insert((gateway, route.network_id), route)?;
		}
		tx.commit()?;
		Ok(())
	}

	async fn deploy_test(&self, gateway: Address, _path: &[u8]) -> Result<(Address, u64)> {
		let mut tester = [0; 32];
		getrandom::getrandom(&mut tester).unwrap();
		let block = block(self.genesis);
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(GATEWAY)?;
			t.insert(tester, gateway)?;
			let mut t = tx.open_multimap_table(TESTERS)?;
			t.insert(gateway, tester)?;
		}
		tx.commit()?;
		Ok((tester, block))
	}

	async fn estimate_message_cost(
		&self,
		_gateway: Address,
		_dest: NetworkId,
		msg_size: usize,
		_gas_limit: u128,
	) -> Result<u128> {
		Ok(msg_size as u128 * 100)
	}

	async fn send_message(&self, addr: Address, mut msg: GmpMessage) -> Result<MessageId> {
		anyhow::ensure!(msg.src_network == self.network_id, "invalid source network id");
		let tx = self.db.begin_write()?;
		let id = {
			// read nonce
			let mut t = tx.open_table(NONCE)?;
			let nonce = t.get((msg.src, addr))?.map(|a| a.value()).unwrap_or_default();
			// increment nonce
			t.insert((msg.src, addr), nonce + 1)?;
			msg.nonce = nonce;
			let id = msg.message_id();

			// read gateway address
			let t = tx.open_table(GATEWAY)?;
			let gateway = t.get(addr)?.context("tester not deployed")?.value();

			// insert gateway event
			let mut t = tx.open_multimap_table(EVENTS)?;
			let block = block(self.genesis);
			t.insert((gateway, block), GmpEvent::MessageReceived(msg))?;

			id
		};
		tx.commit()?;
		Ok(id)
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
	/// Calculate transaction base fee for a chain.
	async fn transaction_base_fee(&self) -> Result<u128> {
		Ok(0)
	}

	/// Returns gas limit of latest block.
	async fn block_gas_limit(&self) -> Result<u64> {
		Ok(0)
	}

	/// Withdraw gateway funds.
	async fn withdraw_funds(
		&self,
		_gateway: Address,
		_amount: u128,
		_address: Address,
	) -> Result<()> {
		Ok(())
	}

	/// Deposit gateway funds.
	async fn deposit_funds(&self, gateway: Address, amount: u128) -> Result<()> {
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(BALANCE)?;
			let balance = read_balance(&t, self.address)?;
			if balance < amount {
				anyhow::bail!("insufficient balance");
			}
			let dest_balance = read_balance(&t, gateway)?;
			t.insert(self.address, balance - amount)?;
			t.insert(gateway, dest_balance + amount)?;
		}
		tx.commit()?;
		Ok(())
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

#[cfg(test)]
mod tests {
	use super::*;
	use time_primitives::MockTssSigner;

	async fn connector(network: NetworkId, mnemonic: u8) -> Result<Connector> {
		Connector::new(ConnectorParams {
			network_id: network,
			blockchain: "rust".to_string(),
			network: network.to_string(),
			url: "tempfile".to_string(),
			mnemonic: mnemonic.to_string(),
			cctp_sender: None,
			cctp_attestation: None,
		})
		.await
	}

	fn gmp_msg(src: Address, dest: Address) -> GmpMessage {
		GmpMessage {
			src_network: 0,
			dest_network: 0,
			src,
			dest,
			nonce: 0,
			gas_limit: 0,
			gas_cost: 0,
			bytes: vec![],
		}
	}

	#[tokio::test]
	async fn smoke_test() -> Result<()> {
		let network = 0;
		let chain = connector(network, 0).await?;
		let shard = MockTssSigner::new(0);
		assert_eq!(chain.balance(chain.address()).await?, 0);
		chain.faucet().await?;
		assert_eq!(chain.balance(chain.address()).await?, FAUCET);
		let (gateway, block) = chain.deploy_gateway("".as_ref(), "".as_ref(), "".as_ref()).await?;
		chain.transfer(gateway, 10_000).await?;
		assert_eq!(chain.balance(gateway).await?, 10_000);
		chain.set_shards(gateway, &[shard.public_key()]).await?;
		assert_eq!(&chain.shards(gateway).await?, &[shard.public_key()]);
		let current = chain.block_stream().next().await.unwrap();
		let events = chain.read_events(gateway, block..current).await?;
		assert_eq!(events, vec![GmpEvent::ShardRegistered(shard.public_key())]);
		let (src, _) = chain.deploy_test(gateway, "".as_ref()).await?;
		let (dest, _) = chain.deploy_test(gateway, "".as_ref()).await?;
		let msg = gmp_msg(src, dest);
		chain.send_message(src, msg.clone()).await?;
		let current2 = chain.block_stream().next().await.unwrap();
		let events = chain.read_events(gateway, current..current2).await?;
		assert_eq!(events, vec![GmpEvent::MessageReceived(msg.clone())]);
		let cmds = GatewayMessage::new(vec![GatewayOp::SendMessage(msg.clone())]);
		let sig = shard.sign_gateway_message(network, gateway, 0, &cmds);
		chain.submit_commands(gateway, 0, cmds, shard.public_key(), sig).await.unwrap();
		let current = chain.block_stream().next().await.unwrap();
		let msgs = chain.recv_messages(dest, current2..current).await?;
		assert_eq!(msgs, vec![msg]);
		Ok(())
	}
}
