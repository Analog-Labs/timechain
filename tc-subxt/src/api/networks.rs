use crate::worker::Tx;
use crate::{metadata, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{ChainName, ChainNetwork, Gateway, Network, NetworkConfig, NetworkId};

impl SubxtClient {
	pub async fn register_network(&self, network: Network) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::RegisterNetwork { network }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn set_network_config(
		&self,
		network: NetworkId,
		config: NetworkConfig,
	) -> Result<()> {
		let (tx, rx) = oneshot::channel();
		self.tx.unbounded_send((Tx::SetNetworkConfig { network, config }, tx))?;
		let tx = rx.await?;
		self.is_success(&tx).await?;
		Ok(())
	}

	pub async fn networks(&self) -> Result<Vec<NetworkId>> {
		let mut networks = vec![];
		let storage = metadata::storage().networks().networks_iter();
		let mut iter = self.client.storage().at_latest().await?.iter(storage).await?;
		while let Some(Ok(kv)) = iter.next().await {
			networks.push(kv.value);
		}
		Ok(networks)
	}

	pub async fn network_name(
		&self,
		network: NetworkId,
	) -> Result<Option<(ChainName, ChainNetwork)>> {
		let runtime_call = metadata::apis().networks_api().get_network(network);
		let data: Option<(ChainName, ChainNetwork)> = self
			.client
			.runtime_api()
			.at_latest()
			.await?
			.call(runtime_call)
			.await?
			.map(|(name, net)| ((*name).clone(), (*net).clone()));
		Ok(data)
	}

	pub async fn network_gateway(&self, network: NetworkId) -> Result<Option<Gateway>> {
		let runtime_call = metadata::apis().networks_api().get_gateway(network);
		let data = self.client.runtime_api().at_latest().await?.call(runtime_call).await?;
		Ok(data)
	}

	pub async fn network_batch_size(&self, network: NetworkId) -> Result<u32> {
		let storage_query = metadata::storage().networks().network_batch_size(network);
		let data = self
			.client
			.storage()
			.at_latest()
			.await?
			//.fetch_or_default(&storage_query)
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_batch_offset(&self, network: NetworkId) -> Result<u32> {
		let storage_query = metadata::storage().networks().network_batch_offset(network);
		let data = self
			.client
			.storage()
			.at_latest()
			.await?
			//.fetch_or_default(&storage_query)
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_batch_gas_limit(&self, network: NetworkId) -> Result<u128> {
		let storage_query = metadata::storage().networks().network_batch_gas_limit(network);
		let data = self
			.client
			.storage()
			.at_latest()
			.await?
			//.fetch_or_default(&storage_query)
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_shard_task_limit(&self, network: NetworkId) -> Result<u32> {
		let storage_query = metadata::storage().networks().network_shard_task_limit(network);
		let data = self
			.client
			.storage()
			.at_latest()
			.await?
			//.fetch_or_default(&storage_query)
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_shard_size(&self, network: NetworkId) -> Result<u16> {
		let storage_query = metadata::storage().networks().network_shard_size(network);
		self.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Shard size not found"))
	}

	pub async fn network_shard_threshold(&self, network: NetworkId) -> Result<u16> {
		let storage_query = metadata::storage().networks().network_shard_threshold(network);
		self.client
			.storage()
			.at_latest()
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Shard size not found"))
	}
}
