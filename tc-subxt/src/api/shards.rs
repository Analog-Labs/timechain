use crate::worker::Tx;
use crate::{metadata_scope, SubxtClient};
use anyhow::{anyhow, Result};
use futures::channel::oneshot;
use time_primitives::{
	AccountId, Commitment, MemberStatus, NetworkId, ShardId, ShardStatus, TssPublicKey,
};

impl SubxtClient {
	/* subxt doesn't support decoding keys, use shard_id_counter for now
	pub async fn shards(&self) -> Result<Vec<ShardId>> {
		let mut shards = vec![];
		metadata_scope!(self.metadata, {
			let storage = metadata::storage().shards().shard_state_iter();
			let mut iter = self.client.storage().at_latest().await?.iter(storage).await?;
			while let Some(Ok(kv)) = iter.next().await {
				shards.push(kv.keys);
			}
		});
		Ok(shards)
	}*/

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

	pub async fn member_shards(&self, account: &AccountId) -> Result<Vec<ShardId>> {
		let account = subxt::utils::Static(account.clone());
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().shards_api().get_shards(account);
			Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
		})
	}

	pub async fn shard_members(&self, shard_id: ShardId) -> Result<Vec<(AccountId, MemberStatus)>> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().shards_api().get_shard_members(shard_id);
			let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
			Ok(data.into_iter().map(|(account, status)| (account.0, status.0)).collect())
		})
	}

	pub async fn shard_threshold(&self, shard_id: ShardId) -> Result<u16> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().shards_api().get_shard_threshold(shard_id);
			Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
		})
	}

	pub async fn shard_status(&self, shard_id: ShardId) -> Result<ShardStatus> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().shards_api().get_shard_status(shard_id);
			let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
			Ok(data.0)
		})
	}

	pub async fn shard_commitment(&self, shard_id: ShardId) -> Result<Option<Commitment>> {
		metadata_scope!(self.metadata, {
			let runtime_call = metadata::apis().shards_api().get_shard_commitment(shard_id);
			let output = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
			let output_converted = output.map(|static_commitment| (*static_commitment).clone());
			Ok(output_converted)
		})
	}

	pub async fn shard_public_key(&self, shard_id: ShardId) -> Result<Option<TssPublicKey>> {
		Ok(self.shard_commitment(shard_id).await?.map(|v| v.0[0]))
	}

	pub async fn submit_commitment(
		&self,
		shard_id: ShardId,
		commitment: Commitment,
		proof_of_knowledge: [u8; 65],
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((
			Tx::Commitment {
				shard_id,
				commitment,
				proof_of_knowledge,
			},
			tx,
		))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}

	pub async fn submit_online(&self, shard_id: ShardId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Ready { shard_id }, tx))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}

	pub async fn force_shard_offline(&self, shard_id: ShardId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::ForceShardOffline { shard_id }, tx))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}

	pub async fn set_shard_config(&self, shard_size: u16, shard_threshold: u16) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx
			.unbounded_send((Tx::SetShardConfig { shard_size, shard_threshold }, tx))?;
		rx.await?.wait_for_success().await?;
		Ok(())
	}

	pub async fn shard_size_config(&self) -> Result<u16> {
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

	pub async fn shard_threshold_config(&self) -> Result<u16> {
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
