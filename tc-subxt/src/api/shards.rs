use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use subxt::utils::H256;
use time_primitives::{
	AccountId, BlockHash, Commitment, MemberStatus, NetworkId, ShardId, ShardStatus, TssPublicKey,
};

impl SubxtClient {
	pub async fn shards(&self, block: BlockHash) -> Result<Vec<ShardId>> {
		let mut shards = vec![];
		let storage = metadata::storage().shards().shards_iter();
		let mut iter = self.client.storage().at(block).iter(storage).await?;
		while let Some(Ok(kv)) = iter.next().await {
			shards.push(kv.value);
		}
		Ok(shards)
	}

	pub async fn shard_network(
		&self,
		shard_id: u64,
		block: BlockHash,
	) -> Result<Option<NetworkId>> {
		let block = H256(block.0);
		let storage_query = metadata::storage().shards().shard_network(shard_id);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn member_shards(
		&self,
		account: &AccountId,
		block: BlockHash,
	) -> Result<Vec<ShardId>> {
		let block = H256(block.0);
		let account = subxt::utils::Static(account.clone());
		let runtime_call = metadata::apis().shards_api().shards(account);
		Ok(self.client.runtime_api().at(block).call(runtime_call).await?)
	}

	pub async fn shard_members(
		&self,
		shard_id: ShardId,
		block: BlockHash,
	) -> Result<Vec<(AccountId, MemberStatus)>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().shards_api().shard_members(shard_id);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(data.into_iter().map(|(account, status)| (account.0, status.0)).collect())
	}

	pub async fn shard_threshold(&self, shard_id: ShardId, block: BlockHash) -> Result<u16> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().shards_api().shard_threshold(shard_id);
		Ok(self.client.runtime_api().at(block).call(runtime_call).await?)
	}

	pub async fn shard_status(&self, shard_id: ShardId, block: BlockHash) -> Result<ShardStatus> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().shards_api().shard_status(shard_id);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(data.0)
	}

	pub async fn shard_commitment(
		&self,
		shard_id: ShardId,
		block: BlockHash,
	) -> Result<Option<Commitment>> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().shards_api().shard_commitment(shard_id);
		let output = self.client.runtime_api().at(block).call(runtime_call).await?;
		let output_converted = output.map(|static_commitment| (*static_commitment).clone());
		Ok(output_converted)
	}

	pub async fn shard_public_key(
		&self,
		shard_id: ShardId,
		block: BlockHash,
	) -> Result<Option<TssPublicKey>> {
		Ok(self.shard_commitment(shard_id, block).await?.map(|v| v.0[0]))
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
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn submit_online(&self, shard_id: ShardId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Ready { shard_id }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn force_shard_offline(&self, shard_id: ShardId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::ForceShardOffline { shard_id }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}
}
