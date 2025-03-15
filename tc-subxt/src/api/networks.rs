use crate::worker::Tx;
use crate::{metadata, BlockHash, SubxtClient};
use anyhow::Result;
use futures::channel::oneshot;
use time_primitives::{
	CctpContracts, CctpUrl, ChainName, ChainNetwork, Gateway, Network, NetworkConfig, NetworkId,
};

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

	pub async fn networks(&self, block: Option<BlockHash>) -> Result<Vec<NetworkId>> {
		let mut networks = vec![];
		let storage = metadata::storage().networks().networks_iter();
		let mut iter = self.st_at_or_latest(block).await?.iter(storage).await?;
		while let Some(Ok(kv)) = iter.next().await {
			networks.push(kv.value);
		}
		Ok(networks)
	}

	pub async fn network_name(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<Option<(ChainName, ChainNetwork)>> {
		let runtime_call = metadata::apis().networks_api().get_network(network);
		let data: Option<(ChainName, ChainNetwork)> = self
			.rt_at_or_latest(block)
			.await?
			.call(runtime_call)
			.await?
			.map(|(name, net)| ((*name).clone(), (*net).clone()));
		Ok(data)
	}

	pub async fn get_cctp_contracts(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<Option<CctpContracts>> {
		let runtime_call = metadata::apis().networks_api().get_cctp_contracts(network);
		let data: Option<CctpContracts> = self
			.rt_at_or_latest(block)
			.await?
			.call(runtime_call)
			.await?
			.map(|contracts| (*contracts).clone());
		Ok(data)
	}

	pub async fn get_cctp_url(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<Option<CctpUrl>> {
		let runtime_call = metadata::apis().networks_api().get_cctp_url(network);
		let data: Option<CctpUrl> = self
			.rt_at_or_latest(block)
			.await?
			.call(runtime_call)
			.await?
			.map(|url| (*url).clone());
		Ok(data)
	}

	pub async fn network_gateway(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<Option<Gateway>> {
		let runtime_call = metadata::apis().networks_api().get_gateway(network);
		let data = self.rt_at_or_latest(block).await?.call(runtime_call).await?;
		Ok(data)
	}

	pub async fn network_batch_size(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<u32> {
		let storage_query = metadata::storage().networks().network_batch_size(network);
		let data = self
			.st_at_or_latest(block)
			.await?
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_batch_offset(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<u32> {
		let storage_query = metadata::storage().networks().network_batch_offset(network);
		let data = self
			.st_at_or_latest(block)
			.await?
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_batch_gas_limit(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<u128> {
		let storage_query = metadata::storage().networks().network_batch_gas_limit(network);
		let data = self
			.st_at_or_latest(block)
			.await?
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_shard_task_limit(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<u32> {
		let storage_query = metadata::storage().networks().network_shard_task_limit(network);
		let data = self
			.st_at_or_latest(block)
			.await?
			.fetch(&storage_query)
			.await?
			.unwrap_or_default();
		Ok(data)
	}

	pub async fn network_shard_size(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<u16> {
		let storage_query = metadata::storage().networks().network_shard_size(network);
		self.st_at_or_latest(block)
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Shard size not found"))
	}

	pub async fn network_shard_threshold(
		&self,
		network: NetworkId,
		block: Option<BlockHash>,
	) -> Result<u16> {
		let storage_query = metadata::storage().networks().network_shard_threshold(network);
		self.st_at_or_latest(block)
			.await?
			.fetch(&storage_query)
			.await?
			.ok_or_else(|| anyhow::anyhow!("Shard size not found"))
	}
}
