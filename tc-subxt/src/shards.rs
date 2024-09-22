use crate::{metadata_scope, SubxtClient};
use anyhow::{anyhow, Result};
use time_primitives::{NetworkId, ShardId, TssPublicKey};

impl SubxtClient {
	pub async fn network_id_counter(&self) -> Result<NetworkId> {
		metadata_scope!(self.metadata, {
			let storage = metadata::storage().networks().network_id_counter();
			Ok(self.client.storage().at_latest().await?.fetch_or_default(&storage).await?)
		})
	}

	pub async fn shard_public_key(&self, shard_id: ShardId) -> Result<TssPublicKey> {
		metadata_scope!(self.metadata, {
			let storage = metadata::storage().shards().shard_commitment(shard_id);
			self.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage)
				.await?
				.ok_or(anyhow!("shard key not found"))
				.map(|v| v[0])
		})
	}

	pub async fn shard_id_counter(&self) -> Result<u64> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().shards().shard_id_counter();
			Ok(self
				.client
				.storage()
				.at_latest()
				.await?
				.fetch_or_default(&storage_query)
				.await?)
		})
	}

	pub async fn shard_network(&self, shard_id: u64) -> Result<NetworkId> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().shards().shard_network(shard_id);
			self.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage_query)
				.await?
				.ok_or(anyhow!("Shard network not found"))
		})
	}

	pub async fn shard_size(&self) -> Result<u16> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().elections().shard_size();
			self.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage_query)
				.await?
				.ok_or(anyhow!("Shard size not found"))
		})
	}

	pub async fn shard_threshold(&self) -> Result<u16> {
		metadata_scope!(self.metadata, {
			let storage_query = metadata::storage().elections().shard_threshold();
			self.client
				.storage()
				.at_latest()
				.await?
				.fetch(&storage_query)
				.await?
				.ok_or(anyhow!("Shard size not found"))
		})
	}
}
