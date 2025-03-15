use crate::worker::Tx;
use crate::{metadata, BlockHash, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{AccountId, BlockNumber, NetworkId, PeerId, PublicKey};

impl SubxtClient {
	pub async fn member_network(
		&self,
		account: &AccountId,
		block: Option<BlockHash>,
	) -> Result<Option<NetworkId>> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_network(&account);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?)
	}

	pub async fn member_peer_id(
		&self,
		account: &AccountId,
		block: Option<BlockHash>,
	) -> Result<Option<PeerId>> {
		let account = subxt::utils::Static(account.clone());
		let runtime_call = metadata::apis().members_api().get_member_peer_id(account);
		let data = self.rt_at_or_latest(block).await?.call(runtime_call).await?;
		Ok(data)
	}

	pub async fn member_online(
		&self,
		account: &AccountId,
		block: Option<BlockHash>,
	) -> Result<bool> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_online(account);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?.is_some())
	}

	pub async fn member_registered(
		&self,
		account: &AccountId,
		block: Option<BlockHash>,
	) -> Result<bool> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_registered(account);
		Ok(self.st_at_or_latest(block).await?.fetch(&storage_query).await?.is_some())
	}

	pub async fn heartbeat_timeout(&self, block: Option<BlockHash>) -> Result<BlockNumber> {
		let runtime_call = metadata::apis().members_api().get_heartbeat_timeout();
		Ok(self.rt_at_or_latest(block).await?.call(runtime_call).await?)
	}

	pub async fn register_member(
		&self,
		network: NetworkId,
		public_key: PublicKey,
		peer_id: PeerId,
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx
			.unbounded_send((Tx::RegisterMember { network, public_key, peer_id }, tx))?;
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
