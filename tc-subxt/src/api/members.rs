use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{AccountId, Balance, BlockNumber, NetworkId, PeerId, PublicKey};

impl SubxtClient {
	pub async fn member_network(&self, account: &AccountId) -> Result<Option<NetworkId>> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_network(&account);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?)
	}

	pub async fn member_peer_id(&self, account: &AccountId) -> Result<Option<PeerId>> {
		let account = subxt::utils::Static(account.clone());
		let runtime_call = metadata::apis().members_api().get_member_peer_id(account);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	pub async fn member_stake(&self, account: &AccountId) -> Result<u128> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_stake(&account);
		Ok(self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.unwrap_or_default())
	}

	pub async fn member_staker(&self, account: &AccountId) -> Result<Option<AccountId>> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_staker(&account);
		Ok(self
			.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.map(|s| s.0))
	}

	pub async fn member_online(&self, account: &AccountId) -> Result<bool> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_online(account);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?.is_some())
	}

	pub async fn member_registered(&self, account: &AccountId) -> Result<bool> {
		let account = subxt::utils::Static(account.clone());
		let storage_query = metadata::storage().members().member_registered(account);
		Ok(self.client.storage().at_latest().await?.fetch(&storage_query).await?.is_some())
	}

	pub async fn heartbeat_timeout(&self) -> Result<BlockNumber> {
		let runtime_call = metadata::apis().members_api().get_heartbeat_timeout();
		Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
	}

	pub async fn min_stake(&self) -> Result<Balance> {
		let runtime_call = metadata::apis().members_api().get_min_stake();
		Ok(self.client.runtime_api().at_latest().await?.call(runtime_call).await?)
	}

	pub async fn register_member(
		&self,
		network: NetworkId,
		public_key: PublicKey,
		peer_id: PeerId,
		stake_amount: u128,
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((
			Tx::RegisterMember {
				network,
				public_key,
				peer_id,
				stake_amount,
			},
			tx,
		))?;
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
