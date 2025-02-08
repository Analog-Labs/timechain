use anyhow::Result;
use redb::{Database, TableDefinition};
use std::collections::VecDeque;
use subxt::utils::H256;

use crate::{timechain_client::ITransactionDbOps, worker::TxData};

const TX_TABLE: TableDefinition<[u8; 64], &[u8]> = TableDefinition::new("pending_txs");

pub struct TransactionsDB {
	db: Database,
	public_key: [u8; 32],
}

impl TransactionsDB {
	pub fn new(path: &str, public_key: [u8; 32]) -> Result<Self> {
		let db = Database::create(path)?;

		let write_tx = db.begin_write()?;
		{
			write_tx.open_table(TX_TABLE)?;
		}
		write_tx.commit()?;
		Ok(Self { db, public_key })
	}
}

impl ITransactionDbOps for TransactionsDB {
	fn store_tx(&self, tx_data: &TxData) -> Result<()> {
		let mut composite_key = [0u8; 64];
		composite_key[..32].copy_from_slice(&self.public_key);
		composite_key[32..].copy_from_slice(tx_data.hash.as_bytes());

		let tx_value = bincode::serialize(tx_data)?;

		let write_tx = self.db.begin_write()?;
		{
			let mut table = write_tx.open_table(TX_TABLE)?;
			table.insert(&composite_key, &*tx_value)?;
		}
		write_tx.commit()?;
		Ok(())
	}

	fn remove_tx(&self, hash: H256) -> Result<()> {
		let mut composite_key = [0u8; 64];
		composite_key[..32].copy_from_slice(&self.public_key);
		composite_key[32..].copy_from_slice(hash.as_bytes());

		let write_tx = self.db.begin_write()?;
		{
			let mut table = write_tx.open_table(TX_TABLE)?;
			table.remove(&composite_key)?;
		}
		write_tx.commit()?;
		Ok(())
	}

	fn load_pending_txs(&self) -> Result<VecDeque<TxData>> {
		let read_tx = self.db.begin_read()?;
		let table = read_tx.open_table(TX_TABLE)?;

		let mut lower_bound = [0u8; 64];
		lower_bound[..32].copy_from_slice(&self.public_key);

		let mut upper_bound = [0xffu8; 64];
		upper_bound[..32].copy_from_slice(&self.public_key);

		let mut pending_txs = Vec::new();
		for entry in table.range(lower_bound..=upper_bound)? {
			let (_, value) = entry?;
			let tx_data: TxData = bincode::deserialize(value.value())?;
			pending_txs.push(tx_data);
		}

		pending_txs.sort_by_key(|tx| tx.nonce);
		Ok(pending_txs.into())
	}
}
