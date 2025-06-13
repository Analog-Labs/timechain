use anyhow::{Context, Result};
use redb::{
	Database, Key, MultimapTableDefinition, ReadableTable, TableDefinition, TypeName, Value,
	WriteTransaction,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;
use time_primitives::{
	Address32, BatchId, GatewayMessage, GatewayOp, GmpEvent, GmpMessage, GmpParams, Hash, IChain,
	IConnect, IConnector, IConnectorAdmin, MessageId, NetworkId, Route, TssPublicKey, TssSignature,
};

const CONFIG: TableDefinition<u64, u64> = TableDefinition::new("config");
const BALANCE: TableDefinition<Address32, u128> = TableDefinition::new("balance");
const ADMIN: TableDefinition<Address32, Address32> = TableDefinition::new("admin");
const NONCE: TableDefinition<(Address32, Address32), u64> = TableDefinition::new("nonce");
const EVENTS: MultimapTableDefinition<(Address32, u64), Bincode<GmpEvent>> =
	MultimapTableDefinition::new("events");
const SHARDS: MultimapTableDefinition<Address32, TssPublicKey> =
	MultimapTableDefinition::new("shards");
const ROUTES: TableDefinition<(Address32, NetworkId), Bincode<Route>> =
	TableDefinition::new("routes");
const GATEWAY: TableDefinition<Address32, Address32> = TableDefinition::new("gateway");
const TESTERS: MultimapTableDefinition<Address32, Address32> =
	MultimapTableDefinition::new("testers");

const BLOCK_KEY: u64 = 0;

#[derive(Clone)]
pub struct Chain {
	network_id: NetworkId,
	address: Address32,
}

fn mnemonic_to_address(mnemonic: &str) -> Address32 {
	*blake3::hash(mnemonic.as_bytes()).as_bytes()
}

impl Chain {
	pub fn new(network_id: NetworkId, mnemonic: &str) -> Self {
		let address = mnemonic_to_address(mnemonic);
		Self { network_id, address }
	}

	pub fn open(self, db: String) -> Result<Connector> {
		Connector::new(self, db)
	}
}

impl IChain for Chain {
	/// Formats an address into a string.
	fn format_address(&self, address: Address32) -> String {
		hex::encode(address)
	}

	/// Parses an address from a string.
	fn parse_address(&self, address: &str) -> Result<Address32> {
		let addr = hex::decode(address).map_err(|_| anyhow::anyhow!("invalid address"))?;
		let addr = addr.try_into().map_err(|_| anyhow::anyhow!("invalid address"))?;
		Ok(addr)
	}

	/// Network identifier.
	fn network_id(&self) -> NetworkId {
		self.network_id
	}

	/// Human readable connector account identifier.
	fn address(&self) -> Address32 {
		self.address
	}
}

#[async_trait::async_trait]
impl IConnect for Chain {
	fn chain(&self) -> &dyn IChain {
		self
	}

	async fn connect(&self, url: String) -> Result<Arc<dyn IConnector>> {
		Ok(Arc::new(Connector::new(self.clone(), url)?))
	}

	async fn connect_admin(&self, url: String) -> Result<Arc<dyn IConnectorAdmin>> {
		Ok(Arc::new(Connector::new(self.clone(), url)?))
	}
}

fn read_balance<T: ReadableTable<Address32, u128>>(table: &T, addr: Address32) -> Result<u128> {
	Ok(if let Some(value) = table.get(addr)? { value.value() } else { 0 })
}

fn read_admin<T: ReadableTable<Address32, Address32>>(
	table: &T,
	gateway: Address32,
) -> Result<Address32> {
	Ok(table.get(gateway)?.context("invalid gateway")?.value())
}

#[derive(Clone)]
pub struct Connector {
	chain: Chain,
	db: Arc<Database>,
	_tmpfile: Option<Arc<NamedTempFile>>,
}

impl Connector {
	pub fn with_mnemonic(&self, mnemonic: &str) -> Self {
		self.with_address(mnemonic_to_address(mnemonic))
	}

	pub fn with_address(&self, address: Address32) -> Self {
		let mut clone = Clone::clone(self);
		clone.chain.address = address;
		clone
	}
}

impl Connector {
	fn new(chain: Chain, url: String) -> Result<Self> {
		let (tmpfile, path) = if url == "tempfile" {
			let file = NamedTempFile::new()?;
			let path = file.path().to_owned();
			(Some(Arc::new(file)), path)
		} else {
			(None, Path::new(&url).to_owned())
		};
		let db = Arc::new(Database::create(path)?);
		let tx = db.begin_write()?;
		tx.open_table(CONFIG)?;
		tx.open_table(BALANCE)?;
		tx.open_table(ADMIN)?;
		tx.open_table(ROUTES)?;
		tx.open_table(GATEWAY)?;
		tx.open_table(NONCE)?;
		tx.open_multimap_table(EVENTS)?;
		tx.open_multimap_table(SHARDS)?;
		tx.open_multimap_table(TESTERS)?;
		tx.commit()?;
		let db2 = db.clone();
		tokio::task::spawn(async move {
			let inc_block = move || {
				let tx = db2.begin_write()?;
				let block = {
					let mut t = tx.open_table(CONFIG)?;
					let block = t.get(BLOCK_KEY)?.map(|v| v.value()).unwrap_or_default() + 1;
					t.insert(BLOCK_KEY, block)?;
					block
				};
				tx.commit()?;
				Ok::<_, anyhow::Error>(block)
			};
			loop {
				match inc_block() {
					Ok(block) => {
						tracing::info!("new block {block}");
					},
					Err(err) => {
						tracing::error!("{err}");
					},
				}
				tokio::time::sleep(Duration::from_secs(6)).await;
			}
		});
		Ok(Self { chain, db, _tmpfile: tmpfile })
	}

	fn ensure_admin(&self, tx: &WriteTransaction, gateway: Address32) -> Result<()> {
		let t = tx.open_table(ADMIN)?;
		let admin = read_admin(&t, gateway)?;
		if admin != self.chain.address {
			anyhow::bail!("not admin");
		}
		Ok(())
	}

	fn transfer_from(
		&self,
		tx: &WriteTransaction,
		from: Address32,
		to: Address32,
		amount: u128,
	) -> Result<()> {
		let mut t = tx.open_table(BALANCE)?;
		let balance = read_balance(&t, from)?;
		if balance < amount {
			anyhow::bail!("insufficient balance");
		}
		let dest_balance = read_balance(&t, to)?;
		t.insert(from, balance - amount)?;
		t.insert(to, dest_balance + amount)?;
		Ok(())
	}

	fn block(&self) -> Result<u64> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(CONFIG)?;
		Ok(t.get(BLOCK_KEY)?.map(|v| v.value()).unwrap_or_default())
	}
}

#[async_trait::async_trait]
impl IConnector for Connector {
	fn chain(&self) -> &dyn IChain {
		&self.chain
	}

	async fn finalized_block(&self) -> Result<u64> {
		self.block()
	}

	/// Reads gmp messages from the target chain.
	async fn read_events(&self, gateway: Address32, blocks: Range<u64>) -> Result<Vec<GmpEvent>> {
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
		gateway: Address32,
		batch: BatchId,
		msg: GatewayMessage,
		signer: TssPublicKey,
		sig: TssSignature,
	) -> Result<(), String> {
		let hash = GmpParams::new(self.chain.network_id, gateway).hash(&msg.hash(batch));

		time_primitives::verify_signature(signer, &hash, sig)
			.map_err(|_| "invalid signature".to_string())?;
		(|| {
			let tx = self.db.begin_write()?;
			{
				let mut events = tx.open_multimap_table(EVENTS)?;
				let mut shards = tx.open_multimap_table(SHARDS)?;
				let block = self.block()?;
				for op in &msg.ops {
					match op {
						GatewayOp::RegisterShard(key, _) => {
							shards.insert(gateway, key)?;
							events.insert((gateway, block), GmpEvent::ShardRegistered(*key))?;
						},
						GatewayOp::UnregisterShard(key, _) => {
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
				events.insert(
					(gateway, block),
					GmpEvent::BatchExecuted { batch_id: batch, tx_hash: None },
				)?;
			}
			tx.commit()?;
			Ok(())
		})()
		.map_err(|err: anyhow::Error| err.to_string())
	}
	/// Get EIP1559 `max_fee_per_gas` estimate for a chain.
	async fn gas_price(&self) -> Result<u128> {
		Ok(1)
	}
}

#[async_trait::async_trait]
impl IConnectorAdmin for Connector {
	async fn faucet(&self, balance: u128) -> Result<()> {
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(BALANCE)?;
			t.insert(self.chain.address, balance)?;
		}
		tx.commit()?;
		Ok(())
	}

	/// Queries the account balance.
	async fn balance(&self, addr: Address32) -> Result<u128> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(BALANCE)?;
		let Some(balance) = t.get(addr)? else {
			return Ok(0);
		};
		Ok(balance.value())
	}

	async fn transfer(&self, address: Address32, amount: u128) -> Result<()> {
		let tx = self.db.begin_write()?;
		self.transfer_from(&tx, self.chain.address, address, amount)?;
		tx.commit()?;
		Ok(())
	}

	async fn deploy_gateway(&self, _proxy: &[u8], _gateway: &[u8]) -> Result<(Address32, u64)> {
		let mut gateway = [0; 32];
		getrandom::fill(&mut gateway).unwrap();
		let block = self.block()?;
		let tx = self.db.begin_write()?;
		{
			let mut t = tx.open_table(ADMIN)?;
			t.insert(gateway, self.chain.address)?;
		}
		tx.commit()?;
		Ok((gateway, block))
	}

	async fn redeploy_gateway(&self, proxy: Address32, _gateway: &[u8]) -> Result<()> {
		let tx = self.db.begin_write()?;
		self.ensure_admin(&tx, proxy)
	}

	async fn admin(&self, gateway: Address32) -> Result<Address32> {
		let tx = self.db.begin_read()?;
		let t = tx.open_table(ADMIN)?;
		let admin = read_admin(&t, gateway)?;
		Ok(admin)
	}

	async fn set_admin(&self, gateway: Address32, new_admin: Address32) -> Result<()> {
		let tx = self.db.begin_write()?;
		self.ensure_admin(&tx, gateway)?;
		let mut t = tx.open_table(ADMIN)?;
		t.insert(gateway, new_admin)?;
		Ok(())
	}

	async fn shards(&self, gateway: Address32) -> Result<Vec<TssPublicKey>> {
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

	async fn set_shards(
		&self,
		gateway: Address32,
		register: &[(TssPublicKey, u16)],
		revoke: &[(TssPublicKey, u16)],
	) -> Result<()> {
		let tx = self.db.begin_write()?;
		{
			self.ensure_admin(&tx, gateway)?;
			let mut events = tx.open_multimap_table(EVENTS)?;
			let mut shards = tx.open_multimap_table(SHARDS)?;
			let block = self.block()?;
			for (key, _) in revoke {
				if shards.remove(gateway, key)? {
					events.insert((gateway, block), GmpEvent::ShardUnregistered(*key))?;
				}
			}
			for (key, _) in register {
				if !shards.insert(gateway, key)? {
					events.insert((gateway, block), GmpEvent::ShardRegistered(*key))?;
				}
			}
		}
		tx.commit()?;
		Ok(())
	}

	async fn routes(&self, gateway: Address32) -> Result<Vec<Route>> {
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

	async fn set_route(&self, gateway: Address32, route: Route) -> Result<()> {
		let tx = self.db.begin_write()?;
		{
			self.ensure_admin(&tx, gateway)?;
			let mut t = tx.open_table(ROUTES)?;
			t.insert((gateway, route.network_id), route)?;
		}
		tx.commit()?;
		Ok(())
	}

	async fn deploy_tester(&self, gateway: Address32, _path: &[u8]) -> Result<(Address32, u64)> {
		let mut tester = [0; 32];
		getrandom::fill(&mut tester).unwrap();
		let block = self.block()?;
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

	async fn estimate_message_gas_limit(
		&self,
		_contract: Address32,
		_src_network: NetworkId,
		_src: Address32,
		_payload: Vec<u8>,
	) -> Result<u64> {
		Ok(100_000)
	}

	async fn estimate_message_cost(
		&self,
		_gateway: Address32,
		_dest_network: NetworkId,
		msg_size: u16,
		gas_limit: u64,
	) -> Result<u128> {
		Ok(gas_limit as u128 + msg_size as u128 * 20 + 100_000)
	}
	async fn send_message(
		&self,
		src: Address32,
		dest_network: NetworkId,
		dest: Address32,
		gas_limit: u64,
		_msg_cost: u128,
		payload: Vec<u8>,
	) -> Result<MessageId> {
		let tx = self.db.begin_write()?;
		let id = {
			// read nonce
			let mut t = tx.open_table(NONCE)?;
			let nonce = t.get((src, dest))?.map(|a| a.value()).unwrap_or_default();
			// construct msg
			let msg = GmpMessage {
				src_network: self.chain.network_id,
				src,
				dest_network,
				dest,
				nonce,
				gas_limit: gas_limit as _,
				bytes: payload,
			};
			let id = msg.message_id();
			// increment nonce
			t.insert((src, dest), nonce + 1)?;

			// read gateway address
			let t = tx.open_table(GATEWAY)?;
			let gateway = t.get(src)?.context("tester not deployed")?.value();

			// insert gateway event
			let mut t = tx.open_multimap_table(EVENTS)?;
			let block = self.block()?;
			t.insert((gateway, block), GmpEvent::MessageReceived(msg))?;
			id
		};
		tx.commit()?;
		Ok(id)
	}

	async fn recv_messages(&self, addr: Address32, blocks: Range<u64>) -> Result<Vec<GmpMessage>> {
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

	/// Returns gas limit of latest block.
	async fn block_gas_limit(&self) -> Result<u64> {
		Ok(u64::MAX)
	}

	/// Withdraw gateway funds.
	async fn withdraw_funds(
		&self,
		gateway: Address32,
		amount: u128,
		address: Address32,
	) -> Result<()> {
		let tx = self.db.begin_write()?;
		self.ensure_admin(&tx, gateway)?;
		self.transfer_from(&tx, gateway, address, amount)?;
		tx.commit()?;
		Ok(())
	}

	/// Debug a transaction.
	async fn debug_transaction(&self, _tx: Hash) -> Result<String> {
		anyhow::bail!("debug_transaction is not supported on this backend");
	}
}

#[derive(Debug)]
pub struct Bincode<T>(pub T);

impl<T> Value for Bincode<T>
where
	T: Debug + Serialize + for<'a> Deserialize<'a>,
{
	type SelfType<'a>
		= T
	where
		Self: 'a;

	type AsBytes<'a>
		= Vec<u8>
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

	async fn connector(network: NetworkId, mnemonic: u8) -> Result<Arc<dyn IConnectorAdmin>> {
		Chain::new(network, &mnemonic.to_string())
			.connect_admin("tempfile".to_string())
			.await
	}

	fn gmp_msg(src: Address32, dest: Address32) -> GmpMessage {
		GmpMessage {
			src_network: 0,
			dest_network: 0,
			src,
			dest,
			nonce: 0,
			gas_limit: 100_000,
			bytes: vec![],
		}
	}

	#[tokio::test]
	async fn smoke_test() -> Result<()> {
		let network = 0;
		let chain = connector(network, 0).await?;
		let shard = MockTssSigner::new(0);
		assert_eq!(chain.balance(chain.chain().address()).await?, 0);
		chain.faucet(100_000).await?;
		assert_eq!(chain.balance(chain.chain().address()).await?, 100_000);
		let (gateway, block) = chain.deploy_gateway("".as_ref(), "".as_ref()).await?;
		chain.redeploy_gateway(gateway, "".as_ref()).await?;
		chain.transfer(gateway, 10_000).await?;
		assert_eq!(chain.balance(gateway).await?, 10_000);
		chain.set_shards(gateway, &[(shard.public_key(), 1)], &[]).await?;
		assert_eq!(&chain.shards(gateway).await?, &[shard.public_key()]);
		tokio::time::sleep(Duration::from_secs(6)).await;
		let current = chain.finalized_block().await.unwrap();
		let events = chain.read_events(gateway, block..current).await?;
		assert_eq!(events, vec![GmpEvent::ShardRegistered(shard.public_key())]);
		let (src, _) = chain.deploy_tester(gateway, "".as_ref()).await?;
		let (dest, _) = chain.deploy_tester(gateway, "".as_ref()).await?;
		let payload = vec![];
		let gas_limit =
			chain.estimate_message_gas_limit(dest, network, src, payload.clone()).await?;
		let msg_cost = chain
			.estimate_message_cost(gateway, network, payload.len() as u16, gas_limit)
			.await?;
		chain.send_message(src, network, dest, gas_limit, msg_cost, payload).await?;
		let msg = gmp_msg(src, dest);
		tokio::time::sleep(Duration::from_secs(6)).await;
		let current2 = chain.finalized_block().await.unwrap();
		let events = chain.read_events(gateway, current..current2).await?;
		assert_eq!(events, vec![GmpEvent::MessageReceived(msg.clone())]);
		let cmds = GatewayMessage::new(vec![GatewayOp::SendMessage(msg.clone())]);
		let sig = shard.sign_gateway_message(network, gateway, 0, &cmds);
		chain.submit_commands(gateway, 0, cmds, shard.public_key(), sig).await.unwrap();
		tokio::time::sleep(Duration::from_secs(6)).await;
		let current = chain.finalized_block().await.unwrap();
		let msgs = chain.recv_messages(dest, current2..current).await?;
		assert_eq!(msgs, vec![msg]);
		Ok(())
	}
}
