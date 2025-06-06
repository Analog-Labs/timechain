use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use subxt::utils::H256;
use time_primitives::{AccountId, BlockHash, BlockNumber, NetworkId, PeerId};

impl SubxtClient {
	pub async fn member_network(
		&self,
		account: &AccountId,
		block: BlockHash,
	) -> Result<Option<NetworkId>> {
		let block = H256(block.0);
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_network(&account);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?)
	}

	pub async fn member_peer_id(
		&self,
		account: &AccountId,
		block: BlockHash,
	) -> Result<Option<PeerId>> {
		let block = H256(block.0);
		let account = subxt::utils::Static(account.clone());
		let runtime_call = metadata::apis().members_api().member_peer_id(account);
		let data = self.client.runtime_api().at(block).call(runtime_call).await?;
		Ok(data)
	}

	pub async fn member_online(&self, account: &AccountId, block: BlockHash) -> Result<bool> {
		let block = H256(block.0);
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_online(account);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?.is_some())
	}

	pub async fn member_registered(&self, account: &AccountId, block: BlockHash) -> Result<bool> {
		let block = H256(block.0);
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_registered(account);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?.is_some())
	}

	pub async fn is_heartbeat_submitted(
		&self,
		account: &AccountId,
		block: BlockHash,
	) -> Result<bool> {
		let block = H256(block.0);
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().heartbeat(account);
		Ok(self.client.storage().at(block).fetch(&storage_query).await?.is_some())
	}

	pub async fn heartbeat_timeout(&self, block: BlockHash) -> Result<BlockNumber> {
		let block = H256(block.0);
		let runtime_call = metadata::apis().members_api().heartbeat_timeout();
		Ok(self.client.runtime_api().at(block).call(runtime_call).await?)
	}

	pub async fn register_member(
		&self,
		network: NetworkId,
		account: AccountId,
		peer_id: PeerId,
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::RegisterMember { network, account, peer_id }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn unregister_member(&self, member: AccountId) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::UnregisterMember { member }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn submit_heartbeat(&self) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::Heartbeat, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}
}
