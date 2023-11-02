use crate::{timechain_runtime, SubxtClient};
use anyhow::{anyhow, Result};
use time_primitives::{ShardId, ShardsPayload, TssPublicKey};
use timechain_runtime::runtime_types::time_primitives::shard;
use timechain_runtime::runtime_types::time_primitives::shard::{Network, ShardStatus};

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
			.ok_or(anyhow!("shard key not found"))?[0]
			.0)
	}

	pub async fn shard_id_counter(&self) -> Result<u64> {
		let storage_query = timechain_runtime::storage().shards().shard_id_counter();
		Ok(self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or(anyhow!("Shard id counter not found"))?)
	}

	pub async fn shard_network(&self, shard_id: u64) -> Result<Network> {
		let storage_query = timechain_runtime::storage().shards().shard_network(shard_id);
		Ok(self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or(anyhow!("Shard network not found"))?)
	}

	pub async fn shard_state(&self, shard_id: u64) -> Result<ShardStatus<u32>> {
		let storage_query = timechain_runtime::storage().shards().shard_state(shard_id);
		Ok(self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or(anyhow!("Shard Status not found"))?)
	}
}

impl ShardsPayload for SubxtClient {
	fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Vec<TssPublicKey>,
		proof_of_knowledge: [u8; 65],
	) -> Vec<u8> {
		let commitment: Vec<shard::TssPublicKey> = unsafe { std::mem::transmute(commitment) };
		let tx = timechain_runtime::tx()
			.shards()
			.commit(shard_id, commitment, proof_of_knowledge);
		self.make_transaction(&tx)
	}

	fn submit_online(&self, shard_id: ShardId) -> Vec<u8> {
		let tx = timechain_runtime::tx().shards().ready(shard_id);
		self.make_transaction(&tx)
	}
}
