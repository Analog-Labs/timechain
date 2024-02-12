use crate::{timechain_runtime, SubxtClient};
use anyhow::{anyhow, Result};
use time_primitives::{NetworkId, ShardId};
use timechain_runtime::runtime_types::time_primitives::shard::ShardStatus;

impl SubxtClient {
	pub async fn shard_public_key(&self, shard_id: ShardId) -> Result<[u8; 33]> {
		let storage = timechain_runtime::storage().shards().shard_commitment(shard_id);
		Ok(self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage)
			.await?
			.ok_or(anyhow!("shard key not found"))?[0])
	}

	pub async fn shard_id_counter(&self) -> Result<u64> {
		let storage_query = timechain_runtime::storage().shards().shard_id_counter();
		let shard_id = self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch_or_default(&storage_query)
			.await?;
		Ok(shard_id)
	}

	pub async fn shard_network(&self, shard_id: u64) -> Result<NetworkId> {
		let storage_query = timechain_runtime::storage().shards().shard_network(shard_id);
		self.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or(anyhow!("Shard network not found"))
	}

	pub async fn shard_state(&self, shard_id: u64) -> Result<ShardStatus<u32>> {
		let storage_query = timechain_runtime::storage().shards().shard_state(shard_id);
		self.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or(anyhow!("Shard Status not found"))
	}
}
